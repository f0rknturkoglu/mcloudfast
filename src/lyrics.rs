//! Lyrics for the playing track, from Spotify, a local file or LRCLIB.
//!
//! [LRCLIB](https://lrclib.net) provides plain and LRC-synced lyrics without an
//! account or key. Spotifast tries Spotify's transcription first when the
//! playback session is signed in, then LRCLIB.
//!
//! Matching starts with an exact lookup, then ranks search results. Track
//! length is the strongest signal for distinguishing versions with the same
//! title.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

const API: &str = "https://lrclib.net/api";
/// Lyrics do not change; a cached answer is good for this long.
const CACHE_LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// A cached "Spotify has none" is good for this long: it may be wrong, and
/// costs a look at LRCLIB every time.
const NONE_LIFETIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// A candidate this far from the playing track's length is another
/// recording, however well the names match.
const MAX_DRIFT_SECS: f64 = 30.0;
/// LRCLIB answers about one request a second and says how long to wait.
/// A search right after a missed exact lookup often lands in that second.
const RATE_LIMIT_RETRIES: u32 = 3;
const SERVICE_RETRIES: u32 = 1;
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(20);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// Any longer and the song may have moved on; the error shows instead,
/// and a click on it asks again.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(5);

/// Track metadata used for lyrics lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// When the line starts; `None` in lyrics that carry no timing.
    pub at_ms: Option<u32>,
    pub text: String,
    /// When the line ends, where the source says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<u32>,
    /// The line's timed pieces, syllables or words, where the source times
    /// them: their texts, joined, are `text`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub syllables: Vec<Syllable>,
}

/// A timed piece of a line: a syllable or a word, with the space after it
/// when it ends one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Syllable {
    pub text: String,
    pub at_ms: u32,
    pub end_ms: u32,
}

/// How a line's pieces were timed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceTiming {
    /// The source timed them.
    Exact,
    /// Only the line was timed: its words fill at a sung pace, and what the
    /// line has over is held on its last word.
    Estimated,
}

/// Every word is sung in at least this long when the time is shared out.
const MIN_LINE_MS: u32 = 600;
/// About how long a sung character takes; a line followed by a long
/// pause is held no longer than its words need.
const MS_PER_CHAR: u32 = 85;
/// A pause before the next line this long is an interlude.
pub const INTERLUDE_MS: u32 = 4_000;
/// A line's estimated fill ends this much before the next line, so the
/// whole line is seen lit while it is still the sung one, rather than
/// completed by the change to the next.
const LINE_TAIL_MS: u32 = 300;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lyrics {
    pub lines: Vec<Line>,
    /// Every line has a time, so the panel can follow the song.
    pub synced: bool,
    /// The database knows there are no words.
    pub instrumental: bool,
    /// Who wrote the song, where Spotify says; shown under the words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writers: Vec<String>,
}

impl Lyrics {
    /// The line being sung at `position_ms`, or `None` before the first one
    /// starts and for lyrics without timing.
    pub fn active_line(&self, position_ms: u32) -> Option<usize> {
        if !self.synced {
            return None;
        }
        let started = self
            .lines
            .partition_point(|line| line.at_ms.is_some_and(|at| at <= position_ms));
        started.checked_sub(1)
    }

    /// When line `index` ends: the source's end, else when the next line
    /// starts, held no longer than its words need before a long pause.
    pub fn line_end(&self, index: usize) -> Option<u32> {
        let line = self.lines.get(index)?;
        let at = line.at_ms?;
        if let Some(end) = line.end_ms.filter(|end| *end > at) {
            return Some(end);
        }
        if let Some(last) = line.syllables.last() {
            return Some(last.end_ms.max(at));
        }
        let chars = line.text.chars().count() as u32;
        let needed = at + MIN_LINE_MS.max(chars * MS_PER_CHAR);
        let next = self.lines[index + 1..].iter().find_map(|line| line.at_ms);
        // Sung on up to the next line, unless a long pause comes first.
        Some(match next {
            Some(next) if next > at && next.saturating_sub(needed) < INTERLUDE_MS => next,
            _ => needed,
        })
    }

    /// Line `index`'s timed pieces: the source's, else its words with the
    /// line's time shared out by their length.
    pub fn pieces(&self, index: usize) -> Option<(Vec<Syllable>, PieceTiming)> {
        let line = self.lines.get(index)?;
        if !line.syllables.is_empty() {
            return Some((line.syllables.clone(), PieceTiming::Exact));
        }
        let at = line.at_ms?;
        let end = self.line_end(index)?;
        // A short line keeps a share of its time rather than all of the tail.
        let tail = (end.saturating_sub(at) / 4).min(LINE_TAIL_MS);
        Some((
            estimate_words(&line.text, at, end - tail),
            PieceTiming::Estimated,
        ))
    }

    /// Where the song is between two lines long apart: the pause's start
    /// and end, while `position_ms` is inside it.
    pub fn interlude(&self, position_ms: u32) -> Option<(u32, u32)> {
        if !self.synced {
            return None;
        }
        let next = self
            .lines
            .iter()
            .position(|line| line.at_ms.is_some_and(|at| at > position_ms))?;
        let start = match next.checked_sub(1) {
            Some(previous) => self.line_end(previous)?,
            None => 0,
        };
        let end = self.lines[next].at_ms?;
        (position_ms >= start && end.saturating_sub(start) >= INTERLUDE_MS).then_some((start, end))
    }
}

/// `text`'s words, each with the space after it, timed from `at` to `end`.
/// The words fill at a sung character's pace by their length; when the line
/// has time to spare, that time is the note it ends on, held.
fn estimate_words(text: &str, at: u32, end: u32) -> Vec<Syllable> {
    let mut words: Vec<String> = Vec::new();
    for piece in text.split_inclusive(' ') {
        match words.last_mut() {
            // Runs of spaces stay with the word before them.
            Some(last) if piece.trim().is_empty() => last.push_str(piece),
            _ => words.push(piece.to_string()),
        }
    }
    if words.is_empty() {
        return Vec::new();
    }
    // Each word carries a little weight of its own, so short words are not
    // swept past, and a vowel drawn out over several letters gains more.
    let weight = |word: &str| word.trim().chars().count() as u32 + 2 + drawn_out(word);
    let weights: Vec<u32> = words.iter().map(|word| weight(word)).collect();
    let total: u32 = weights.iter().sum::<u32>().max(1);
    let sung = text.chars().filter(|c| !c.is_whitespace()).count().max(1) as u32;
    let natural = sung * MS_PER_CHAR;
    let span = end.saturating_sub(at);
    // A line sung quicker than its words need is squeezed evenly; one with
    // time to spare fills at a character's pace and holds what is left.
    let pace = if span < natural {
        span as f64 / f64::from(total)
    } else {
        f64::from(natural) / f64::from(total)
    };
    let mut start = at;
    let mut done = 0;
    words
        .into_iter()
        .zip(&weights)
        .enumerate()
        .map(|(index, (word, weight))| {
            done += weight;
            let stop = if index + 1 == weights.len() {
                end
            } else {
                at.saturating_add((f64::from(done) * pace).round() as u32)
                    .min(end)
            }
            .max(start);
            let piece = Syllable {
                text: word,
                at_ms: start,
                end_ms: stop,
            };
            start = stop;
            piece
        })
        .collect()
}

/// How much more time a word's vowels are drawn out over: each letter past
/// the first in a run of the same vowel, up to a point.
fn drawn_out(word: &str) -> u32 {
    let mut extra = 0;
    let mut previous: Option<char> = None;
    let mut run = 0;
    for c in word.chars() {
        let c = c.to_ascii_lowercase();
        if is_vowel(c) && Some(c) == previous {
            run += 1;
            if run > 1 && extra < 3 {
                extra += 1;
                run = 1;
            }
        } else {
            run = 0;
        }
        previous = Some(c);
    }
    extra
}

/// Whether `c` is a vowel letters are drawn out with.
fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'à'
            | 'á'
            | 'â'
            | 'ã'
            | 'ä'
            | 'å'
            | 'æ'
            | 'è'
            | 'é'
            | 'ê'
            | 'ë'
            | 'ì'
            | 'í'
            | 'î'
            | 'ï'
            | 'ò'
            | 'ó'
            | 'ô'
            | 'õ'
            | 'ö'
            | 'ø'
            | 'œ'
            | 'ù'
            | 'ú'
            | 'û'
            | 'ü'
            | 'ý'
            | 'ÿ'
            | 'ı'
            | 'İ'
    )
}

/// Parses Spotify's `color-lyrics` response.
pub fn from_spotify(json: &serde_json::Value) -> Option<Lyrics> {
    let lyrics = json.get("lyrics")?;
    let sync = lyrics.get("syncType").and_then(|value| value.as_str());
    let synced = matches!(sync, Some("LINE_SYNCED" | "SYLLABLE_SYNCED"));
    let lines: Vec<Line> = lyrics
        .get("lines")?
        .as_array()?
        .iter()
        .filter_map(|line| {
            let text = line.get("words")?.as_str()?.trim();
            if text.is_empty() || text == "\u{266a}" {
                return None;
            }
            let at_ms = spotify_ms(line.get("startTimeMs")).filter(|_| synced);
            let end_ms = spotify_ms(line.get("endTimeMs"))
                .filter(|end| synced && at_ms.is_some_and(|at| *end > at));
            let syllables = if synced {
                spotify_syllables(line, text, end_ms)
            } else {
                Vec::new()
            };
            Some(Line {
                at_ms,
                text: text.to_string(),
                end_ms,
                syllables,
            })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    let synced = synced && lines.iter().all(|line| line.at_ms.is_some());
    let writers = lyrics
        .get("songWriters")
        .and_then(|value| value.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|name| name.as_str())
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some(Lyrics {
        lines,
        synced,
        instrumental: false,
        writers,
    })
}

/// Who wrote the song, from Spotify's credits view: the names under every
/// role that says who wrote it, in Spotify's order and without repeats.
pub fn writers_from_credits(json: &serde_json::Value) -> Vec<String> {
    let Some(roles) = json.get("roleCredits").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    let mut writers: Vec<String> = Vec::new();
    let mut titled = false;
    for role in roles {
        let title = role
            .get("roleTitle")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_lowercase();
        if title.is_empty() {
            continue;
        }
        titled = true;
        if title.contains("writ") || title.contains("compos") || title.contains("lyric") {
            add_writers(role, &mut writers);
        }
    }
    // Some answers name no role at all. Spotify's clients read them by
    // position, the writers second after the performers.
    if !titled && let Some(role) = roles.get(1) {
        add_writers(role, &mut writers);
    }
    writers
}

/// Adds the names under `role` to `writers`, each once.
fn add_writers(role: &serde_json::Value, writers: &mut Vec<String>) {
    let Some(names) = role.get("artists").and_then(|value| value.as_array()) else {
        return;
    };
    for name in names.iter().filter_map(|name| name.get("name")) {
        let Some(name) = name.as_str().map(str::trim) else {
            continue;
        };
        if name.is_empty() || writers.iter().any(|seen| seen.eq_ignore_ascii_case(name)) {
            continue;
        }
        writers.push(name.to_string());
    }
}

/// Spotify writes its times as strings of digits, sometimes as numbers.
fn spotify_ms(value: Option<&serde_json::Value>) -> Option<u32> {
    let value = value?;
    value
        .as_str()
        .and_then(|text| text.parse().ok())
        .or_else(|| value.as_u64().and_then(|n| u32::try_from(n).ok()))
}

/// A Spotify line's syllables, where it times them: each starts at its
/// `startTimeMs` and covers the next `numChars` characters of the line (or
/// its own text), ending where the next one starts. A list that does not
/// cover the line exactly is left out, and the line is timed as a whole.
fn spotify_syllables(line: &serde_json::Value, text: &str, end_ms: Option<u32>) -> Vec<Syllable> {
    let Some(list) = line.get("syllables").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    let chars: Vec<char> = text.chars().collect();
    let mut taken = 0;
    let mut pieces: Vec<Syllable> = Vec::new();
    for syllable in list {
        let Some(at_ms) = spotify_ms(syllable.get("startTimeMs")) else {
            return Vec::new();
        };
        let piece: String = match syllable
            .get("numChars")
            .or_else(|| syllable.get("charsCount"))
            .and_then(|count| count.as_u64())
        {
            Some(count) if taken + count as usize <= chars.len() => {
                chars[taken..taken + count as usize].iter().collect()
            }
            Some(_) => return Vec::new(),
            None => match syllable
                .get("words")
                .or_else(|| syllable.get("text"))
                .and_then(|value| value.as_str())
            {
                Some(own) => own.to_string(),
                None => return Vec::new(),
            },
        };
        taken += piece.chars().count();
        let end = spotify_ms(syllable.get("endTimeMs")).filter(|end| *end > at_ms);
        if let Some(last) = pieces.last_mut()
            && last.end_ms <= last.at_ms
        {
            last.end_ms = at_ms.max(last.at_ms);
        }
        pieces.push(Syllable {
            text: piece,
            at_ms,
            end_ms: end.unwrap_or(0),
        });
    }
    // The rest of the line (spaces, a closing mark) stays with the last.
    if taken < chars.len()
        && let Some(last) = pieces.last_mut()
    {
        last.text.extend(&chars[taken..]);
        taken = chars.len();
    }
    if pieces.is_empty() || taken != chars.len() {
        return Vec::new();
    }
    if let Some(last) = pieces.last_mut()
        && last.end_ms <= last.at_ms
    {
        last.end_ms = end_ms.unwrap_or(last.at_ms + MIN_LINE_MS).max(last.at_ms);
    }
    pieces
}

/// The cached answer at `path`, while it is fresh.
pub fn cached(path: &Path) -> Option<Option<Lyrics>> {
    let answer = read_cache(&path.to_path_buf())?;
    if answer.is_none() && older_than(path, NONE_LIFETIME) {
        return None;
    }
    Some(answer)
}

/// Whether the file at `path` was written over `age` ago.
fn older_than(path: &Path, age: Duration) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_none_or(|elapsed| elapsed > age)
}

/// Remember an answer at `path`, `None` included.
pub fn store(path: &Path, found: &Option<Lyrics>) {
    write_cache(path, found);
}

/// A previously found transcription still helps during an outage. An old
/// negative answer must never turn a failed request into "no lyrics".
pub fn previously_found(path: &Path) -> Option<Lyrics> {
    read_cache_contents(path).flatten()
}

/// Fetches lyrics for `query`, using the disk cache when available.
pub async fn fetch(
    http: &reqwest::Client,
    cache_dir: &Path,
    query: &Query,
) -> Result<Option<Lyrics>> {
    fetch_from(http, API, cache_dir, query).await
}

async fn fetch_from(
    http: &reqwest::Client,
    api: &str,
    cache_dir: &Path,
    query: &Query,
) -> Result<Option<Lyrics>> {
    let cache_path = cache_dir.join(format!("{}.json", cache_key(query)));
    if let Some(cached) = read_cache(&cache_path) {
        return Ok(cached);
    }
    let result = tokio::time::timeout(LOOKUP_TIMEOUT, lookup(http, api, query))
        .await
        .context("lyrics service did not respond in time")
        .and_then(|result| result);
    let found = match result {
        Ok(found) => found,
        Err(error) => return previously_found(&cache_path).map(Some).ok_or(error),
    };
    write_cache(&cache_path, &found);
    Ok(found)
}

async fn lookup(http: &reqwest::Client, api: &str, query: &Query) -> Result<Option<Lyrics>> {
    let title = clean_title(&query.title);
    let artist = clean_artist(&query.artist);
    if title.is_empty() || artist.is_empty() {
        return Ok(None);
    }
    let duration = (query.duration_ms / 1000).to_string();
    let mut exact = vec![
        ("artist_name", artist.as_str()),
        ("track_name", title.as_str()),
    ];
    if !query.album.trim().is_empty() {
        exact.push(("album_name", query.album.trim()));
    }
    if query.duration_ms > 0 {
        exact.push(("duration", duration.as_str()));
    }
    let exact_error = match get_from::<Record>(http, api, "/get", &exact).await {
        Ok(Some(record)) => {
            if let Some(lyrics) = record.lyrics() {
                return Ok(Some(lyrics));
            }
            None
        }
        Ok(None) => None,
        Err(error) => {
            // LRCLIB's exact lookup and search can fail independently. A
            // transient exact failure is not proof that search has no lyrics.
            let Some(wait) = search_after(&error) else {
                return Err(error);
            };
            tokio::time::sleep(wait).await;
            Some(error)
        }
    };
    let candidates = get_from::<Vec<Record>>(
        http,
        api,
        "/search",
        &[
            ("artist_name", artist.as_str()),
            ("track_name", title.as_str()),
        ],
    )
    .await?
    .unwrap_or_default();
    let found = pick(candidates, query).and_then(|record| record.lyrics());
    match (found, exact_error) {
        (None, Some(error)) => Err(error),
        (found, _) => Ok(found),
    }
}

#[derive(Debug)]
struct RequestFailure {
    status: reqwest::StatusCode,
    wait: Duration,
}

impl std::fmt::Display for RequestFailure {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "LRCLIB answered {}", self.status)
    }
}

impl std::error::Error for RequestFailure {}

fn service_busy(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 502..=504)
}

fn search_after(error: &anyhow::Error) -> Option<Duration> {
    if let Some(failure) = error.downcast_ref::<RequestFailure>() {
        return (service_busy(failure.status) && failure.wait <= MAX_RETRY_AFTER)
            .then_some(failure.wait);
    }
    error
        .downcast_ref::<reqwest::Error>()
        .filter(|error| error.is_connect() || error.is_timeout())
        .map(|_| Duration::from_secs(1))
}

fn retry_after(headers: &reqwest::header::HeaderMap) -> Duration {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .map_or(Duration::from_secs(1), |value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .map_or(Duration::MAX, Duration::from_secs)
        })
}

/// One LRCLIB request. A 404 is "nobody has transcribed this" and a 400 is
/// "that is not a question I can answer"; neither is a fault worth showing.
/// Retry short overloads, but stop when the requested wait exceeds our
/// budget. Never shorten Retry-After, including an unparsed HTTP date.
async fn get_from<T: DeserializeOwned>(
    http: &reqwest::Client,
    api: &str,
    path: &str,
    params: &[(&str, &str)],
) -> Result<Option<T>> {
    let mut attempt = 0;
    let response = loop {
        let response = http
            .get(format!("{api}{path}"))
            .query(params)
            .header("Accept", "application/json")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("cannot reach LRCLIB")?;
        let retries = if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            RATE_LIMIT_RETRIES
        } else if service_busy(response.status()) {
            SERVICE_RETRIES
        } else {
            0
        };
        let wait = retry_after(response.headers());
        if attempt >= retries || wait > MAX_RETRY_AFTER {
            break response;
        }
        attempt += 1;
        tokio::time::sleep(wait).await;
    };
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::BAD_REQUEST {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(RequestFailure {
            status,
            wait: retry_after(response.headers()),
        }
        .into());
    }
    Ok(Some(
        response
            .json()
            .await
            .context("unexpected answer from LRCLIB")?,
    ))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Record {
    track_name: String,
    artist_name: String,
    duration: Option<f64>,
    instrumental: bool,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
}

impl Record {
    fn synced(&self) -> Option<&str> {
        self.synced_lyrics
            .as_deref()
            .filter(|text| !text.trim().is_empty())
    }

    fn plain(&self) -> Option<&str> {
        self.plain_lyrics
            .as_deref()
            .filter(|text| !text.trim().is_empty())
    }

    fn lyrics(&self) -> Option<Lyrics> {
        if self.instrumental {
            return Some(Lyrics {
                instrumental: true,
                ..Lyrics::default()
            });
        }
        if let Some(synced) = self.synced() {
            let lines = parse_lrc(synced);
            if !lines.is_empty() {
                return Some(Lyrics {
                    lines,
                    synced: true,
                    instrumental: false,
                    writers: Vec::new(),
                });
            }
        }
        let plain = self.plain()?;
        Some(Lyrics {
            lines: plain
                .lines()
                .map(|line| Line {
                    at_ms: None,
                    text: line.trim_end().to_string(),
                    ..Line::default()
                })
                .collect(),
            synced: false,
            instrumental: false,
            writers: Vec::new(),
        })
    }
}

/// How well a search result fits what is playing; `None` rules it out.
fn score(record: &Record, query: &Query) -> Option<i64> {
    if !loose_match(&record.track_name, &clean_title(&query.title)) {
        return None;
    }
    let mut score = 0;
    if loose_match(&record.artist_name, &clean_artist(&query.artist)) {
        score += 1000;
    }
    if query.duration_ms > 0
        && let Some(duration) = record.duration.filter(|duration| *duration > 0.0)
    {
        let drift = (duration - f64::from(query.duration_ms) / 1000.0).abs();
        if drift > MAX_DRIFT_SECS {
            return None;
        }
        score += ((MAX_DRIFT_SECS - drift) * 10.0) as i64;
    }
    // Timing is the point, so a synced upload wins a tie.
    if record.synced().is_some() {
        score += 200;
    } else if record.plain().is_some() {
        score += 50;
    }
    Some(score)
}

fn pick(records: Vec<Record>, query: &Query) -> Option<Record> {
    let mut best: Option<(i64, Record)> = None;
    for record in records {
        let Some(score) = score(&record, query) else {
            continue;
        };
        if best.as_ref().is_none_or(|(held, _)| score > *held) {
            best = Some((score, record));
        }
    }
    best.map(|(_, record)| record)
}

// ---- names ----------------------------------------------------------------

/// Words a title carries in brackets that a lyrics database does not.
const BRACKET_NOISE: &[&str] = &[
    "remaster",
    "remastered",
    "remix",
    "live",
    "acoustic",
    "version",
    "edit",
    "mix",
    "mono",
    "stereo",
    "deluxe",
    "bonus",
    "expanded",
    "explicit",
    "anniversary",
    "feat",
    "featuring",
    "with",
];

/// What a " - " suffix says when it is not part of the title.
const SUFFIX_NOISE: &[&str] = &[
    "remaster",
    "remastered",
    "radio edit",
    "single version",
    "album version",
    "live",
    "mono",
    "stereo",
    "rerecorded",
    "re-recorded",
];

/// Invisible characters that carry no meaning: a zero-width space, the word
/// joiner and the invisible operators, and a byte order mark. Not the
/// zero-width joiner or non-joiner, which hold emoji sequences and Indic
/// and Persian words together.
fn is_disposable(c: char) -> bool {
    matches!(c, '\u{200b}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}

/// Whether `text` contains `phrase` as whole words, case-insensitively.
fn has_phrase(text: &str, phrase: &str) -> bool {
    let words: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|word| !word.is_empty())
        .map(|word| word.trim_matches('-').to_string())
        .collect();
    let wanted: Vec<&str> = phrase.split(' ').collect();
    words
        .windows(wanted.len())
        .any(|window| window.iter().zip(&wanted).all(|(a, b)| a == b))
}

/// Strips what a player adds to a title and a database leaves out:
/// "(Remastered 2011)", " - Live at Wembley", "feat. Someone".
pub fn clean_title(title: &str) -> String {
    let original: String = title.chars().filter(|c| !is_disposable(*c)).collect();
    let mut cleaned = String::with_capacity(original.len());
    let mut rest = original.as_str();
    while let Some(open) = rest.find(['(', '[']) {
        let closer = if rest[open..].starts_with('(') {
            ')'
        } else {
            ']'
        };
        let Some(close) = rest[open..].find(closer) else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        let noisy = BRACKET_NOISE.iter().any(|word| has_phrase(inner, word));
        if noisy {
            cleaned.push_str(rest[..open].trim_end());
        } else {
            cleaned.push_str(&rest[..open + close + 1]);
        }
        rest = &rest[open + close + 1..];
    }
    cleaned.push_str(rest);
    let mut cleaned = cleaned.trim().to_string();
    // " - Remastered 2009" and friends, from the first dash whose tail is
    // noise; a dash inside a real title stays.
    let mut cut = None;
    for (index, _) in cleaned.match_indices(" - ") {
        let tail = &cleaned[index + 3..];
        if SUFFIX_NOISE.iter().any(|phrase| has_phrase(tail, phrase))
            || tail
                .split_whitespace()
                .next()
                .is_some_and(|word| word.len() == 4 && word.chars().all(|c| c.is_ascii_digit()))
                && has_phrase(tail, "version")
        {
            cut = Some(index);
            break;
        }
    }
    if let Some(index) = cut {
        cleaned.truncate(index);
    }
    let cleaned = strip_featuring(cleaned.trim());
    if cleaned.is_empty() {
        original.trim().to_string()
    } else {
        cleaned
    }
}

/// Everything from a standalone "feat", "ft", or "featuring" on.
fn strip_featuring(text: &str) -> String {
    // The marker is looked for in a lower-cased copy and the cut is made in
    // `text`, and lower casing does not preserve length: Turkish 'İ' becomes
    // two characters and one byte longer, capital sharp s one byte shorter.
    // An offset measured in the copy therefore means somewhere else in the
    // title -- letters kept that should have gone, or, when it lands inside a
    // character, a panic. `starts` carries every offset in the copy back to
    // the same place in `text`, so no offset is ever used in the string it
    // was not measured in.
    let mut lower = String::with_capacity(text.len());
    let mut starts: Vec<usize> = Vec::with_capacity(text.len() + 1);
    for (at, character) in text.char_indices() {
        for lowered in character.to_lowercase() {
            for _ in 0..lowered.len_utf8() {
                starts.push(at);
            }
            lower.push(lowered);
        }
    }
    starts.push(text.len());
    let mut cut = None;
    for marker in ["featuring", "feat", "ft"] {
        let mut from = 0;
        while let Some(found) = lower[from..].find(marker) {
            let start = from + found;
            let end = start + marker.len();
            let before = lower[..start].trim_end_matches(['-', '(']).trim_end();
            let preceded = before.len() < start && !before.is_empty();
            let followed = lower[end..]
                .strip_prefix('.')
                .unwrap_or(&lower[end..])
                .starts_with(' ');
            if preceded && followed {
                let at = before.len();
                cut = Some(cut.map_or(at, |held: usize| held.min(at)));
                break;
            }
            from = end;
        }
    }
    match cut {
        Some(at) => text[..starts[at]].trim().to_string(),
        None => text.trim().to_string(),
    }
}

/// Players report collaborations in ways a database does not file them, and
/// LRCLIB itself sometimes stores "TOOL;Tool" for one artist.
pub fn clean_artist(artist: &str) -> String {
    let original: String = artist.chars().filter(|c| !is_disposable(*c)).collect();
    let cleaned = strip_featuring(&original);
    let cleaned = cleaned.split(';').next().unwrap_or_default().trim();
    if cleaned.is_empty() {
        original.trim().to_string()
    } else {
        cleaned.to_string()
    }
}

/// Lowercase, accents folded, punctuation gone, one space between words.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        let folded = fold(c);
        if folded == '\'' || folded == '’' || folded == '`' {
            continue;
        }
        if folded == '&' {
            for word in [' ', 'a', 'n', 'd', ' '] {
                push_normalized(&mut out, &mut space, word);
            }
            continue;
        }
        push_normalized(&mut out, &mut space, folded);
    }
    out.trim().to_string()
}

fn push_normalized(out: &mut String, space: &mut bool, c: char) {
    if c.is_alphanumeric() {
        out.extend(c.to_lowercase());
        *space = false;
    } else if !*space {
        out.push(' ');
        *space = true;
    }
}

/// The plain letter behind the Latin accents titles most often carry.
fn fold(c: char) -> char {
    match c {
        'À'..='Å' | 'à'..='å' => 'a',
        'Ç' | 'ç' => 'c',
        'È'..='Ë' | 'è'..='ë' => 'e',
        'Ì'..='Ï' | 'ì'..='ï' => 'i',
        'Ñ' | 'ñ' => 'n',
        'Ò'..='Ö' | 'Ø' | 'ò'..='ö' | 'ø' => 'o',
        'Ù'..='Ü' | 'ù'..='ü' => 'u',
        'Ý' | 'ý' | 'ÿ' => 'y',
        'ß' => 's',
        _ => c,
    }
}

fn loose_match(left: &str, right: &str) -> bool {
    let a = normalize(left);
    let b = normalize(right);
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a == b || a.contains(&b) || b.contains(&a)
}

/// Lyrics from text a file holds: timed when its lines carry LRC stamps,
/// plain lines otherwise. `None` for text without words.
pub fn from_text(text: &str) -> Option<Lyrics> {
    if text.trim().is_empty() {
        return None;
    }
    let timed = parse_lrc(text);
    if !timed.is_empty() {
        return Some(Lyrics {
            lines: timed,
            synced: true,
            instrumental: false,
            writers: Vec::new(),
        });
    }
    Some(Lyrics {
        lines: text
            .trim()
            .lines()
            .map(|line| Line {
                at_ms: None,
                text: line.trim_end().to_string(),
                ..Line::default()
            })
            .collect(),
        synced: false,
        instrumental: false,
        writers: Vec::new(),
    })
}

// ---- LRC ------------------------------------------------------------------

/// A TTML time: seconds (`12.5`, `12.5s`) or `m:ss.xx` or `h:mm:ss.xx`, in ms.
fn ttml_time(text: &str) -> Option<u32> {
    let text = text.trim().trim_end_matches('s');
    let mut seconds = 0.0_f64;
    for part in text.split(':') {
        seconds = seconds * 60.0 + part.parse::<f64>().ok()?;
    }
    Some((seconds * 1000.0).round() as u32)
}

/// The value of `name="…"` in an XML tag's attributes.
fn ttml_attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    Some(&tag[start..start + end])
}

fn ttml_text(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Lyrics from TTML, as Apple Music and Spicy Lyrics write them: one `<p>` a
/// line with its times, and `<span>`s for the words when they are timed.
pub fn from_ttml(text: &str) -> Option<Lyrics> {
    let mut lines = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("<p ") {
        let body = &rest[open..];
        let tag_end = body.find('>')?;
        let tag = &body[..tag_end];
        let close = body.find("</p>")?;
        let inner = &body[tag_end + 1..close];
        rest = &body[close + 4..];
        let (Some(at), end) = (
            ttml_attribute(tag, "begin").and_then(ttml_time),
            ttml_attribute(tag, "end").and_then(ttml_time),
        ) else {
            continue;
        };
        let mut syllables = Vec::new();
        let mut words = String::new();
        let mut cursor = inner;
        while let Some(span) = cursor.find("<span") {
            words.push_str(&ttml_text(&cursor[..span]));
            let piece = &cursor[span..];
            let span_tag_end = piece.find('>')?;
            let span_tag = &piece[..span_tag_end];
            let span_close = piece.find("</span>")?;
            let word = ttml_text(&piece[span_tag_end + 1..span_close]);
            if let (Some(word_at), Some(word_end)) = (
                ttml_attribute(span_tag, "begin").and_then(ttml_time),
                ttml_attribute(span_tag, "end").and_then(ttml_time),
            ) {
                // A space between spans belongs to the word before it, as
                // the pieces joined must spell the line.
                if !syllables.is_empty() && words.ends_with(' ') {
                    let last: &mut Syllable = syllables.last_mut()?;
                    last.text.push(' ');
                }
                syllables.push(Syllable {
                    text: word.clone(),
                    at_ms: word_at,
                    end_ms: word_end,
                });
            }
            words.push_str(&word);
            cursor = &piece[span_close + 7..];
        }
        words.push_str(&ttml_text(cursor));
        lines.push(Line {
            at_ms: Some(at),
            text: words.trim().to_string(),
            end_ms: end,
            syllables,
        });
    }
    (!lines.is_empty()).then_some(Lyrics {
        lines,
        synced: true,
        instrumental: false,
        writers: Vec::new(),
    })
}

/// Parses `[mm:ss.xx]` lines. A line may open with several stamps when the
/// same words repeat; tags such as `[ar:...]` carry no digits and are
/// skipped. The result is sorted by time.
pub fn parse_lrc(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim_start();
        let mut times = Vec::new();
        while let Some(stamp) = leading_stamp(rest) {
            times.push(stamp.0);
            rest = &rest[stamp.1..];
        }
        if times.is_empty() {
            continue;
        }
        let body = rest.trim();
        let (text, syllables) = word_stamps(body);
        for at_ms in times {
            // A repeated line's word times belong to its first singing.
            let shift = syllables
                .first()
                .map(|first| at_ms as i64 - first.at_ms as i64);
            lines.push(Line {
                at_ms: Some(at_ms),
                text: text.clone(),
                end_ms: None,
                syllables: syllables
                    .iter()
                    .map(|piece| Syllable {
                        text: piece.text.clone(),
                        at_ms: (piece.at_ms as i64 + shift.unwrap_or(0)).max(0) as u32,
                        end_ms: (piece.end_ms as i64 + shift.unwrap_or(0)).max(0) as u32,
                    })
                    .collect(),
            });
        }
    }
    lines.sort_by_key(|line| line.at_ms);
    lines
}

/// An enhanced LRC body, `<00:12.30>Some <00:12.80>words<00:13.40>`: its
/// plain text and its words, each timed from its stamp to the next. A body
/// without stamps gives its text and no words.
fn word_stamps(body: &str) -> (String, Vec<Syllable>) {
    if !body.contains('<') {
        return (body.to_string(), Vec::new());
    }
    let mut text = String::new();
    let mut pieces: Vec<Syllable> = Vec::new();
    let mut rest = body;
    let mut open: Option<u32> = None;
    let mut current = String::new();
    loop {
        let stamp = rest
            .strip_prefix('<')
            .and_then(|inner| inner.find('>').map(|close| (inner, close)))
            .and_then(|(inner, close)| {
                leading_stamp(&format!("[{}]", &inner[..close])).map(|(ms, _)| (ms, close + 2))
            });
        match stamp {
            Some((ms, length)) => {
                if let Some(at) = open
                    && !current.is_empty()
                {
                    pieces.push(Syllable {
                        text: std::mem::take(&mut current),
                        at_ms: at,
                        end_ms: ms.max(at),
                    });
                }
                current.clear();
                open = Some(ms);
                rest = &rest[length..];
            }
            None => {
                let Some(next) = rest.chars().next() else {
                    break;
                };
                text.push(next);
                current.push(next);
                rest = &rest[next.len_utf8()..];
            }
        }
    }
    // Words after the last stamp have no end: they take the whole text's
    // last stamp as theirs, held as long as the word before.
    if let Some(at) = open
        && !current.is_empty()
    {
        let held = pieces
            .last()
            .map_or(MIN_LINE_MS, |last| last.end_ms - last.at_ms);
        pieces.push(Syllable {
            text: current,
            at_ms: at,
            end_ms: at + held.max(1),
        });
    }
    let trimmed = text.trim().to_string();
    if pieces.is_empty()
        || pieces
            .iter()
            .map(|piece| piece.text.as_str())
            .collect::<String>()
            .trim()
            != trimmed
    {
        return (trimmed, Vec::new());
    }
    // Text before the first stamp joins the first word.
    let lead = text.len() - text.trim_start().len();
    if lead > 0 {
        pieces[0].text = pieces[0].text.trim_start().to_string();
    }
    if let Some(last) = pieces.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    (trimmed, pieces)
}

/// A timestamp at the head of `text`: its milliseconds and its length.
fn leading_stamp(text: &str) -> Option<(u32, usize)> {
    let inner = text.strip_prefix('[')?;
    let close = inner.find(']')?;
    let stamp = &inner[..close];
    let (minutes, seconds) = stamp.split_once(':')?;
    let (seconds, fraction) = match seconds.find(['.', ':']) {
        Some(dot) => (&seconds[..dot], Some(&seconds[dot + 1..])),
        None => (seconds, None),
    };
    let minutes: u32 = minutes.parse().ok()?;
    let seconds: u32 = seconds.parse().ok()?;
    let fraction_ms = match fraction {
        Some(digits) if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) => {
            let value: u32 = digits.chars().take(3).collect::<String>().parse().ok()?;
            value * 10u32.pow(3u32.saturating_sub(digits.len().min(3) as u32))
        }
        Some(_) => return None,
        None => 0,
    };
    Some((minutes * 60_000 + seconds * 1_000 + fraction_ms, close + 2))
}

// ---- cache ------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Cached {
    found: Option<Lyrics>,
}

fn cache_key(query: &Query) -> String {
    let digest = Sha1::digest(
        format!(
            "{}|{}|{}|{}",
            query.artist, query.title, query.album, query.duration_ms
        )
        .as_bytes(),
    );
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn read_cache(path: &PathBuf) -> Option<Option<Lyrics>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    if modified.elapsed().unwrap_or(CACHE_LIFETIME) >= CACHE_LIFETIME {
        return None;
    }
    read_cache_contents(path)
}

fn read_cache_contents(path: &Path) -> Option<Option<Lyrics>> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Cached>(&text)
        .ok()
        .map(|cached| cached.found)
}

fn write_cache(path: &Path, found: &Option<Lyrics>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&Cached {
        found: found.clone(),
    }) {
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A local LRCLIB that gives each request, in turn, one of `answers`.
    async fn lrclib(
        answers: Vec<impl Into<String>>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let answers: Vec<String> = answers.into_iter().map(Into::into).collect();
        let server = tokio::spawn(async move {
            let mut paths = Vec::new();
            for answer in answers {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0u8; 4096];
                let len = socket.read(&mut bytes).await.unwrap();
                assert!(len > 0);
                paths.push(
                    String::from_utf8_lossy(&bytes[..len])
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .split('?')
                        .next()
                        .unwrap()
                        .to_owned(),
                );
                socket.write_all(answer.as_bytes()).await.unwrap();
            }
            paths
        });
        (format!("http://{address}"), server)
    }

    const LIMITED: &str = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const UNAVAILABLE: &str = "HTTP/1.1 503 Service Unavailable\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    fn answer(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn outage_query() -> Query {
        Query {
            artist: "Artist".into(),
            title: "Song".into(),
            album: "Album".into(),
            duration_ms: 120_000,
        }
    }

    #[tokio::test]
    async fn search_recovers_when_the_exact_lookup_stays_unavailable() {
        let found = answer(
            r#"[{"trackName":"Song","artistName":"Artist","duration":120,"syncedLyrics":"[00:01.00] A test line"}]"#,
        );
        let (api, server) =
            lrclib(vec![UNAVAILABLE.to_owned(), UNAVAILABLE.to_owned(), found]).await;
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let lyrics = lookup(&http, &api, &outage_query()).await.unwrap().unwrap();
        assert!(lyrics.synced);
        assert_eq!(lyrics.lines[0].text, "A test line");
        assert_eq!(server.await.unwrap(), ["/get", "/get", "/search"]);
    }

    #[tokio::test]
    async fn a_short_503_is_retried_before_search() {
        let found = answer(r#"{"plainLyrics":"A test line"}"#);
        let (api, server) = lrclib(vec![UNAVAILABLE.to_owned(), found]).await;
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        assert!(
            lookup(&http, &api, &outage_query())
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(server.await.unwrap(), ["/get", "/get"]);
    }

    #[tokio::test]
    async fn an_empty_search_after_a_failed_exact_lookup_is_not_a_cached_miss() {
        let (api, server) = lrclib(vec![
            UNAVAILABLE.to_owned(),
            UNAVAILABLE.to_owned(),
            answer("[]"),
        ])
        .await;
        let root =
            std::env::temp_dir().join(format!("spotifast-lyrics-miss-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        assert!(
            fetch_from(&http, &api, &root, &outage_query())
                .await
                .is_err()
        );
        assert!(
            !root
                .join(format!("{}.json", cache_key(&outage_query())))
                .exists()
        );
        assert_eq!(server.await.unwrap(), ["/get", "/get", "/search"]);
    }

    #[tokio::test]
    async fn outages_keep_old_lyrics_without_renewing_their_cache_or_reusing_old_misses() {
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let root =
            std::env::temp_dir().join(format!("spotifast-lyrics-stale-{}", std::process::id()));
        let path = root.join(format!("{}.json", cache_key(&outage_query())));
        for found in [from_text("[00:01.00] A test line"), None] {
            write_cache(&path, &found);
            let stale = std::time::SystemTime::now() - CACHE_LIFETIME - Duration::from_secs(60);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(std::fs::FileTimes::new().set_modified(stale))
                .unwrap();
            assert!(read_cache(&path).is_none());
            let (api, server) = lrclib(vec![UNAVAILABLE; 4]).await;
            let result = fetch_from(&http, &api, &root, &outage_query()).await;
            if found.is_some() {
                assert_eq!(result.unwrap().unwrap().lines[0].text, "A test line");
            } else {
                assert!(result.is_err());
            }
            assert!(
                std::fs::metadata(&path)
                    .unwrap()
                    .modified()
                    .unwrap()
                    .elapsed()
                    .unwrap()
                    > CACHE_LIFETIME
            );
            assert_eq!(
                server.await.unwrap(),
                ["/get", "/get", "/search", "/search"]
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn a_long_retry_after_is_not_shortened_or_bypassed_by_search() {
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        for wait in ["60", "Thu, 29 Oct 2099 12:00:00 GMT"] {
            let response = format!(
                "HTTP/1.1 503 Service Unavailable\r\nRetry-After: {wait}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let (api, server) = lrclib(vec![response]).await;
            let result =
                tokio::time::timeout(Duration::from_secs(1), lookup(&http, &api, &outage_query()))
                    .await
                    .unwrap();
            assert!(result.unwrap_err().to_string().contains("503"));
            assert_eq!(server.await.unwrap(), ["/get"]);
        }
    }

    #[tokio::test]
    async fn a_busy_lrclib_is_asked_again_when_it_says() {
        let found = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]";
        let (api, server) = lrclib(vec![LIMITED, LIMITED, found]).await;
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let answer = get_from::<Vec<Record>>(&http, &api, "/search", &[]).await;
        assert!(matches!(answer, Ok(Some(records)) if records.is_empty()));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn a_lrclib_that_stays_busy_is_an_error() {
        let answers = vec![LIMITED; RATE_LIMIT_RETRIES as usize + 1];
        let (api, server) = lrclib(answers).await;
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let answer = get_from::<Vec<Record>>(&http, &api, "/search", &[]).await;
        let error = answer.expect_err("still limited");
        assert!(error.to_string().contains("429"), "{error}");
        server.await.unwrap();
    }

    #[test]
    fn a_featuring_is_cut_where_it_starts_whatever_the_case_costs() {
        // The cut is found in a lower-cased copy and applied to the original,
        // and lower casing is not always the same number of bytes. Turkish
        // 'İ' grows by one byte, so every offset past it points one byte too
        // far into the title, and the letters between are kept.
        assert_eq!(
            clean_title("\u{130}\u{130} feat. Someone"),
            "\u{130}\u{130}"
        );
        assert_eq!(clean_artist("\u{130}zel feat. Someone"), "\u{130}zel");

        // Capital sharp s shrinks by one, so the offset points one byte SHORT
        // -- and one byte short of the end of a character is a panic.
        assert_eq!(
            clean_title("\u{1e9e} caf\u{e9} feat. Someone"),
            "\u{1e9e} caf\u{e9}"
        );

        // The ASCII case, which already worked, still does.
        assert_eq!(clean_title("Song feat. Someone"), "Song");
        assert_eq!(clean_title("Song ft. Someone"), "Song");
        assert_eq!(clean_artist("Artist featuring Other"), "Artist");
    }

    #[test]
    fn titles_lose_what_a_database_leaves_out() {
        assert_eq!(clean_title("Song (Remastered 2011)"), "Song");
        assert_eq!(clean_title("Song - Live at Wembley"), "Song");
        assert_eq!(clean_title("Song - 2009 Remaster"), "Song");
        assert_eq!(clean_title("Song (feat. Someone)"), "Song");
        assert_eq!(clean_title("Song feat. Someone"), "Song");
        assert_eq!(clean_title("Song ft. Someone"), "Song");
        assert_eq!(clean_title("Song (Part One)"), "Song (Part One)");
        assert_eq!(clean_title("Hyphen - Ated"), "Hyphen - Ated");
        assert_eq!(clean_title("(Remastered)"), "(Remastered)", "never empty");
        assert_eq!(clean_title("Feat\u{200b}ure"), "Feature");
    }

    #[test]
    fn artists_keep_their_first_name() {
        assert_eq!(clean_artist("TOOL;Tool"), "TOOL");
        assert_eq!(clean_artist("Artist feat. Guest"), "Artist");
        assert_eq!(clean_artist("Artist (feat. Guest)"), "Artist");
        assert_eq!(clean_artist("Beyoncé"), "Beyoncé");
    }

    #[test]
    fn matching_is_loose_about_case_accents_and_punctuation() {
        assert!(loose_match("Beyoncé", "beyonce"));
        assert!(loose_match("Rock & Roll", "rock and roll"));
        assert!(loose_match("Don't Stop", "dont stop"));
        assert!(loose_match("Song (Live)", "Song"));
        assert!(!loose_match("Something", "Else"));
        assert!(!loose_match("", "Else"));
    }

    fn record(track: &str, artist: &str, duration: f64, synced: bool) -> Record {
        Record {
            track_name: track.into(),
            artist_name: artist.into(),
            duration: Some(duration),
            instrumental: false,
            synced_lyrics: synced.then(|| "[00:01.00] a".to_string()),
            plain_lyrics: Some("a".to_string()),
        }
    }

    #[test]
    fn the_closest_length_wins_and_synced_breaks_ties() {
        let query = Query {
            artist: "Artist".into(),
            title: "Song".into(),
            album: String::new(),
            duration_ms: 200_000,
        };
        let picked = pick(
            vec![
                record("Song", "Artist", 260.0, true),
                record("Song", "Artist", 201.0, false),
                record("Song", "Artist", 202.0, true),
                record("Other", "Artist", 200.0, true),
            ],
            &query,
        )
        .unwrap();
        assert_eq!(picked.duration, Some(202.0));
        assert!(pick(vec![record("Song", "Artist", 400.0, true)], &query).is_none());
    }

    #[test]
    fn word_timed_lrc_keeps_its_words() {
        let lines = parse_lrc("[00:12.30]<00:12.30>Hello <00:12.80>world<00:13.40>");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello world");
        let words: Vec<_> = lines[0]
            .syllables
            .iter()
            .map(|piece| (piece.text.as_str(), piece.at_ms, piece.end_ms))
            .collect();
        assert_eq!(
            words,
            [("Hello ", 12_300, 12_800), ("world", 12_800, 13_400)]
        );
        // Plain LRC has no words of its own.
        assert!(parse_lrc("[00:01.00]Plain line")[0].syllables.is_empty());
    }

    #[test]
    fn spotify_syllables_cover_their_line() {
        let json = serde_json::json!({"lyrics": {"syncType": "SYLLABLE_SYNCED", "lines": [
            {"startTimeMs": "1000", "endTimeMs": "2000", "words": "Hi there",
             "syllables": [{"startTimeMs": "1000", "numChars": 3}, {"startTimeMs": "1400", "numChars": 5}]},
            {"startTimeMs": "3000", "words": "Next", "syllables": [{"startTimeMs": "3000", "numChars": 9}]}
        ]}});
        let lyrics = from_spotify(&json).unwrap();
        assert!(lyrics.synced);
        let first: Vec<_> = lyrics.lines[0]
            .syllables
            .iter()
            .map(|piece| (piece.text.as_str(), piece.at_ms, piece.end_ms))
            .collect();
        assert_eq!(first, [("Hi ", 1000, 1400), ("there", 1400, 2000)]);
        // Counts that overrun the line leave it timed as a whole.
        assert!(lyrics.lines[1].syllables.is_empty());
    }

    #[test]
    fn a_line_timed_only_as_a_whole_shares_its_time_by_length() {
        let lyrics = Lyrics {
            lines: vec![
                Line {
                    at_ms: Some(1_000),
                    text: "one three".into(),
                    ..Line::default()
                },
                Line {
                    at_ms: Some(3_000),
                    text: "next".into(),
                    ..Line::default()
                },
            ],
            synced: true,
            instrumental: false,
            writers: Vec::new(),
        };
        let (pieces, timing) = lyrics.pieces(0).unwrap();
        assert_eq!(timing, PieceTiming::Estimated);
        let words: Vec<_> = pieces
            .iter()
            .map(|piece| (piece.text.as_str(), piece.at_ms, piece.end_ms))
            .collect();
        // "one " weighs 3 + 2, "three" 5 + 2, over the 680 ms its eight
        // characters need: the last 300 ms before the next line are left over
        // so the line is seen lit whole, and "three" holds the rest.
        assert_eq!(words, [("one ", 1_000, 1_283), ("three", 1_283, 2_700)]);
    }

    #[test]
    fn words_fill_at_a_sung_pace_and_the_spare_time_is_held() {
        // A vowel drawn out over several letters weighs more than one.
        assert!(drawn_out("loooove") > drawn_out("love"));
        assert_eq!(drawn_out("love"), 0);

        // A line with time to spare fills at a character's pace (12 characters
        // at 85 ms) and holds the rest on its last word.
        let lyrics = Lyrics {
            lines: vec![
                Line {
                    at_ms: Some(0),
                    text: "Von, Von, uh".into(),
                    ..Line::default()
                },
                Line {
                    at_ms: Some(4_370),
                    text: "next".into(),
                    ..Line::default()
                },
            ],
            synced: true,
            instrumental: false,
            writers: Vec::new(),
        };
        let (pieces, timing) = lyrics.pieces(0).unwrap();
        assert_eq!(timing, PieceTiming::Estimated);
        let times: Vec<_> = pieces
            .iter()
            .map(|piece| (piece.text.trim(), piece.at_ms, piece.end_ms))
            .collect();
        // The fill ends 300 ms before the next line: 4 070 ms, of which the
        // words need 850, leaves the last word a 3 432 ms note.
        assert_eq!(
            times,
            [("Von,", 0, 319), ("Von,", 319, 638), ("uh", 638, 4_070)]
        );
    }

    #[test]
    fn a_quick_line_squeezes_its_words_evenly() {
        let lyrics = Lyrics {
            lines: vec![
                Line {
                    at_ms: Some(0),
                    text: "one two three four".into(),
                    ..Line::default()
                },
                Line {
                    at_ms: Some(1_000),
                    text: "next".into(),
                    ..Line::default()
                },
            ],
            synced: true,
            instrumental: false,
            writers: Vec::new(),
        };
        let (pieces, _) = lyrics.pieces(0).unwrap();
        let span = pieces.last().unwrap().end_ms - pieces[0].at_ms;
        // Four syllables need more than the line's time, so every word is
        // shortened by the same amount and the last ends where the fill does.
        assert_eq!(span, 750);
        assert!(
            pieces
                .windows(2)
                .all(|pair| pair[0].end_ms == pair[1].at_ms)
        );
    }

    #[test]
    fn a_long_pause_ends_the_line_and_is_an_interlude() {
        let lyrics = Lyrics {
            lines: vec![
                Line {
                    at_ms: Some(1_000),
                    text: "short".into(),
                    ..Line::default()
                },
                Line {
                    at_ms: Some(20_000),
                    text: "later".into(),
                    ..Line::default()
                },
            ],
            synced: true,
            instrumental: false,
            writers: Vec::new(),
        };
        // Held as long as its words need, not until the next line.
        assert_eq!(lyrics.line_end(0), Some(1_000 + 600));
        assert_eq!(lyrics.interlude(10_000), Some((1_600, 20_000)));
        assert_eq!(lyrics.interlude(1_200), None);
        assert_eq!(lyrics.interlude(500), None);
    }

    #[test]
    fn lrc_lines_parse_and_sort() {
        let lines = parse_lrc(
            "[ar:Someone]\n[00:12.50]First\n[00:05]Early\n[01:00.1][02:00.123]Twice\n\nNo stamp\n",
        );
        let times: Vec<u32> = lines.iter().filter_map(|line| line.at_ms).collect();
        assert_eq!(times, vec![5_000, 12_500, 60_100, 120_123]);
        assert_eq!(lines[0].text, "Early");
        assert_eq!(lines[2].text, "Twice");
        assert_eq!(lines[3].text, "Twice");
    }

    #[test]
    fn the_active_line_is_the_last_one_started() {
        let lyrics = Lyrics {
            lines: parse_lrc("[00:05]a\n[00:10]b\n[00:15]c"),
            synced: true,
            instrumental: false,
            writers: Vec::new(),
        };
        assert_eq!(lyrics.active_line(0), None);
        assert_eq!(lyrics.active_line(5_000), Some(0));
        assert_eq!(lyrics.active_line(12_000), Some(1));
        assert_eq!(lyrics.active_line(99_000), Some(2));
        let plain = Lyrics {
            lines: vec![Line::default()],
            synced: false,
            instrumental: false,
            writers: Vec::new(),
        };
        assert_eq!(plain.active_line(5_000), None);
    }

    #[test]
    fn the_credits_view_names_the_writers() {
        let credits = serde_json::json!({
            "trackUri": "spotify:track:x",
            "roleCredits": [
                {
                    "roleTitle": "Performed by",
                    "artists": [{ "name": "The Wanderers" }],
                },
                {
                    "roleTitle": "Written by",
                    "artists": [{ "name": "Alex Rivera" }, { "name": "Sam Okafor" }],
                },
                {
                    "roleTitle": "Lyrics by",
                    "artists": [{ "name": "alex rivera" }, { "name": "  " }],
                },
                { "roleTitle": "Produced by", "artists": [{ "name": "Kai Lund" }] },
            ],
        });
        // Only the writers, once each, in Spotify's order.
        assert_eq!(
            writers_from_credits(&credits),
            ["Alex Rivera".to_string(), "Sam Okafor".to_string()]
        );
        assert!(writers_from_credits(&serde_json::json!({})).is_empty());
        // Without role titles, the roles stand in Spotify's order: performers
        // first, writers second.
        let untitled = serde_json::json!({
            "roleCredits": [
                { "artists": [{ "name": "The Wanderers" }] },
                { "artists": [{ "name": "Alex Rivera" }] },
                { "artists": [{ "name": "Kai Lund" }] },
            ],
        });
        assert_eq!(writers_from_credits(&untitled), ["Alex Rivera".to_string()]);
    }

    #[test]
    fn spotify_lyrics_carry_their_writers_when_it_names_them() {
        let json = serde_json::json!({
            "lyrics": {
                "syncType": "LINE_SYNCED",
                "songWriters": ["Alex Rivera", " "],
                "lines": [{ "startTimeMs": "1000", "words": "a line" }],
            }
        });
        let lyrics = from_spotify(&json).unwrap();
        assert_eq!(lyrics.writers, ["Alex Rivera".to_string()]);
        let quiet = serde_json::json!({
            "lyrics": {
                "syncType": "LINE_SYNCED",
                "lines": [{ "startTimeMs": "1000", "words": "a line" }],
            }
        });
        assert!(from_spotify(&quiet).unwrap().writers.is_empty());
    }

    #[test]
    fn records_turn_into_lyrics() {
        let mut instrumental = record("Song", "Artist", 100.0, false);
        instrumental.instrumental = true;
        assert!(instrumental.lyrics().unwrap().instrumental);
        let synced = record("Song", "Artist", 100.0, true).lyrics().unwrap();
        assert!(synced.synced);
        let plain = record("Song", "Artist", 100.0, false).lyrics().unwrap();
        assert!(!plain.synced);
        assert_eq!(plain.lines.len(), 1);
        let nothing = Record::default();
        assert!(nothing.lyrics().is_none());
    }
}
