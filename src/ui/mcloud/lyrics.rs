//! Word-by-word lyrics for mcloud, in the full-window view and the side
//! panel, in the manner of Apple Music (and of Spicy Lyrics, which mcloud
//! users run beside it):
//! every line is quiet text, and the line being sung lights up word by
//! word, each word filling from its start to its end with a soft edge,
//! with Spicy Lyrics' glow: the word being sung blooms, a word sung out
//! keeps a little of it, and a line with no word times glows as a whole,
//! swelling to its middle. A long pause between lines shows three dots
//! that fill as it passes.
//!
//! The words' times come from [`crate::lyrics::Lyrics::pieces`]: the
//! source's own where it gives them, else the line's time shared out by
//! word length.

use std::sync::Arc;

use egui::{Color32, Galley, Pos2, Rect, pos2, vec2};

use crate::lyrics::{PieceTiming, Syllable};

/// Quiet text's opacity: lines not being sung, and the words of the sung
/// line still to come.
pub(in crate::ui) const QUIET: f32 = 0.38;
/// An idle line's size, as Spicy Lyrics draws the lines not being sung.
pub(in crate::ui) const IDLE_SCALE: f32 = 0.95;
/// The soft edge of the light, in font sizes: Spicy Lyrics' `FadeEm`.
const EDGE: f32 = 0.55;
/// Strips the soft edge is drawn in, enough that the steps do not show.
const EDGE_STEPS: usize = 16;
/// A glyph this close to a piece's edge still counts as reached, so f32
/// rounding in the glyph widths cannot stop the light at a word's end.
const SLACK: f32 = 0.01;
/// Glow copies drawn round a sung word or line: rings of them at growing
/// distance, standing in for a text-shadow's blur.
const GLOW_RINGS: usize = 4;
const GLOW_STEPS: usize = 12;
/// Opacity of each glow copy; their overlaps build the bloom.
const GLOW_COPY: f32 = 0.06;
/// A sung word keeps this share of its glow once it is over.
const SUNG_GLOW: f32 = 0.28;
/// A line's last word bloomed harder, as Spicy Lyrics' does.
const LAST_WORD_GLOW: f32 = 1.5;
/// How far a glow reaches at rest and at its strongest, in font sizes:
/// Spicy Lyrics' text-shadow is 4 px to 12 px at its type size.
const LINE_GLOW_EM: (f32, f32) = (0.06, 0.32);
const WORD_GLOW_EM: (f32, f32) = (0.05, 0.20);

/// How far each piece has been sung at `position_ms`, 0 to 1.
pub(in crate::ui) fn progress(pieces: &[Syllable], position_ms: u32) -> Vec<f32> {
    pieces
        .iter()
        .map(|piece| {
            if position_ms >= piece.end_ms.max(piece.at_ms) {
                1.0
            } else if position_ms <= piece.at_ms {
                0.0
            } else {
                (position_ms - piece.at_ms) as f32 / (piece.end_ms - piece.at_ms).max(1) as f32
            }
        })
        .collect()
}

/// How many of the laid-out glyphs each piece owns. Layout may leave out
/// characters (a space where a line wraps), so the glyphs are matched to the
/// pieces' characters in order rather than counted. `None` when a glyph
/// matches no character left.
fn glyphs_per_piece(glyphs: &[char], pieces: &[Syllable]) -> Option<Vec<usize>> {
    let mut chars = pieces
        .iter()
        .enumerate()
        .flat_map(|(index, piece)| piece.text.chars().map(move |c| (index, c)));
    let mut counts = vec![0; pieces.len()];
    for glyph in glyphs {
        loop {
            let (piece, c) = chars.next()?;
            if c == *glyph {
                counts[piece] += 1;
                break;
            }
            // Only spaces may go missing.
            if !c.is_whitespace() {
                return None;
            }
        }
    }
    Some(counts)
}

/// Where the light ends on each row: the x (from the row's left) up to
/// which its glyphs are lit, given each row's glyph widths, how many
/// characters each piece covers, and how far each piece has been sung.
/// `None` when the pieces don't cover the glyphs one for one.
fn lit_extents(rows: &[Vec<f32>], piece_chars: &[usize], progress: &[f32]) -> Option<Vec<f32>> {
    let glyphs: usize = rows.iter().map(Vec::len).sum();
    if glyphs != piece_chars.iter().sum::<usize>() {
        return None;
    }
    // Each glyph's piece, and how much of its piece's width lies before it.
    let mut owner = Vec::with_capacity(glyphs);
    for (piece, count) in piece_chars.iter().enumerate() {
        owner.extend(std::iter::repeat_n(piece, *count));
    }
    let widths: Vec<f32> = rows.iter().flatten().copied().collect();
    let mut piece_width = vec![0.0f32; piece_chars.len()];
    for (glyph, width) in widths.iter().enumerate() {
        piece_width[owner[glyph]] += width;
    }
    let mut before = vec![0.0f32; piece_chars.len()];
    let mut extents = Vec::with_capacity(rows.len());
    let mut glyph = 0;
    let mut done = false;
    for row in rows {
        let mut lit = 0.0;
        for width in row {
            let piece = owner[glyph];
            if !done {
                let sung = progress[piece].clamp(0.0, 1.0) * piece_width[piece];
                let reach = sung - before[piece];
                // Summing f32 widths leaves the last glyph of a sung-out piece
                // a millionth of a pixel short of its width. Without the
                // slack the light stops there for good, at the first word it
                // finishes, and the words after it never light.
                if reach >= *width - SLACK {
                    lit += width;
                } else {
                    lit += reach.max(0.0);
                    done = true;
                }
            }
            before[piece] += width;
            glyph += 1;
        }
        extents.push(lit);
    }
    Some(extents)
}

/// A line's timed words, where the song is, and how strongly it is lit.
pub(in crate::ui) struct Sung<'a> {
    pub pieces: &'a [Syllable],
    pub timing: PieceTiming,
    pub position_ms: u32,
    /// The font size, which the soft edge and the glow follow.
    pub size: f32,
    /// Fades the light in and out as the line becomes the sung one.
    pub opacity: f32,
}

/// Paints the sung part of a line laid out quiet at `pos`: Spicy Lyrics'
/// glow under the words the light has reached, then `bright`, the same text
/// in the full colour, lit word by word over them. Returns false when the
/// pieces don't match the laid-out text, and the caller lights the line
/// whole.
pub(in crate::ui) fn paint(
    painter: &egui::Painter,
    pos: Pos2,
    bright: &Arc<Galley>,
    sung: &Sung<'_>,
) -> bool {
    let Sung {
        pieces,
        timing,
        position_ms,
        size,
        opacity,
    } = *sung;
    let rows: Vec<Vec<f32>> = bright
        .rows
        .iter()
        .map(|row| {
            row.row
                .glyphs
                .iter()
                .map(|glyph| glyph.advance_width)
                .collect()
        })
        .collect();
    let glyph_chars: Vec<char> = bright
        .rows
        .iter()
        .flat_map(|row| row.row.glyphs.iter().map(|glyph| glyph.chr))
        .collect();
    let Some(piece_chars) = glyphs_per_piece(&glyph_chars, pieces) else {
        return false;
    };
    let progress = progress(pieces, position_ms);
    let Some(extents) = lit_extents(&rows, &piece_chars, &progress) else {
        return false;
    };
    if opacity <= 0.0 {
        return true;
    }
    let lit = Lit {
        pieces,
        piece_chars: &piece_chars,
        progress: &progress,
        extents: &extents,
        timing,
        position_ms,
        size,
        opacity,
    };
    glow(painter, pos, bright, &lit);
    let edge = size * EDGE;
    let clip = painter.clip_rect();
    for (row, lit) in bright.rows.iter().zip(&extents) {
        if *lit <= 0.0 {
            continue;
        }
        let rect = row.rect().translate(pos.to_vec2());
        let left = rect.left() + row.row.glyphs.first().map_or(0.0, |glyph| glyph.pos.x);
        let end = left + lit;
        let full = row.row.glyphs.last().is_some_and(|glyph| {
            *lit >= glyph.pos.x + glyph.advance_width - row.row.glyphs[0].pos.x - 0.5
        });
        // The lit part, then the soft edge as strips fading out.
        let solid_end = if full {
            rect.right() + 1.0
        } else {
            end - edge / 2.0
        };
        let mut solid = painter.with_clip_rect(
            Rect::from_min_max(rect.min, pos2(solid_end.max(rect.left()), rect.bottom()))
                .intersect(clip),
        );
        solid.set_opacity(opacity);
        solid.galley(pos, bright.clone(), Color32::WHITE);
        if !full {
            let step = edge / EDGE_STEPS as f32;
            for index in 0..EDGE_STEPS {
                let x0 = end - edge / 2.0 + step * index as f32;
                let fade = 1.0 - (index as f32 + 0.5) / EDGE_STEPS as f32;
                let mut strip = painter.with_clip_rect(
                    Rect::from_min_max(
                        pos2(x0.max(rect.left()), rect.top()),
                        pos2(x0 + step, rect.bottom()),
                    )
                    .intersect(clip),
                );
                strip.set_opacity(opacity * fade);
                strip.galley(pos, bright.clone(), Color32::WHITE);
            }
        }
    }
    true
}

/// A line's words as they stand at the song's position: what is sung, how
/// far, and how strongly the line is lit.
struct Lit<'a> {
    pieces: &'a [Syllable],
    piece_chars: &'a [usize],
    progress: &'a [f32],
    extents: &'a [f32],
    timing: PieceTiming,
    position_ms: u32,
    size: f32,
    opacity: f32,
}

/// Spicy Lyrics' glow. A line timed only as a whole swells to its middle as
/// it is sung, its bloom following the words the light has reached; a line
/// with timed words glows on the word being sung, and faintly on the one
/// before it, so the light leaves a wake.
fn glow(painter: &egui::Painter, pos: Pos2, bright: &Arc<Galley>, lit: &Lit<'_>) {
    match lit.timing {
        PieceTiming::Estimated => {
            let (Some(first), Some(last)) = (lit.pieces.first(), lit.pieces.last()) else {
                return;
            };
            let span = last.end_ms.saturating_sub(first.at_ms).max(1);
            let through = lit.position_ms.saturating_sub(first.at_ms) as f32 / span as f32;
            let strength = line_glow(through) * lit.opacity;
            let blur = glow_radius(lit.size, LINE_GLOW_EM, strength);
            for (row, extent) in bright.rows.iter().zip(lit.extents) {
                if *extent <= 0.0 {
                    continue;
                }
                let rect = row.rect().translate(pos.to_vec2());
                let left = rect.left() + row.row.glyphs.first().map_or(0.0, |glyph| glyph.pos.x);
                let area =
                    Rect::from_min_max(rect.min, pos2(left + extent, rect.bottom())).expand(blur);
                halo(painter, pos, bright, area, blur, strength);
            }
        }
        PieceTiming::Exact => {
            if let Some(current) = lit
                .progress
                .iter()
                .position(|done| *done > 0.0 && *done < 1.0)
            {
                let last = current + 1 == lit.pieces.len();
                let strength = word_glow(lit.progress[current])
                    * lit.opacity
                    * if last { LAST_WORD_GLOW } else { 1.0 };
                word_halo(painter, pos, bright, lit, current, strength);
            }
            if let Some(sung) = lit.progress.iter().rposition(|done| *done >= 1.0) {
                word_halo(painter, pos, bright, lit, sung, SUNG_GLOW * lit.opacity);
            }
        }
    }
}

/// The bloom round one word: a text-shadow's blur is drawn in the word's
/// own box, so nothing of the words around it is repeated into it.
fn word_halo(
    painter: &egui::Painter,
    pos: Pos2,
    bright: &Arc<Galley>,
    lit: &Lit<'_>,
    word: usize,
    strength: f32,
) {
    let first: usize = lit.piece_chars[..word].iter().sum();
    let Some(area) = glyph_area(bright, first, lit.piece_chars[word]) else {
        return;
    };
    let blur = glow_radius(lit.size, WORD_GLOW_EM, strength);
    let area = area.translate(pos.to_vec2()).expand(blur);
    halo(painter, pos, bright, area, blur, strength);
}

/// How far a glow of `strength` reaches, in font sizes: Spicy Lyrics'
/// text-shadow runs from 4 to 12 px as the glow rises.
fn glow_radius(size: f32, (rest, peak): (f32, f32), strength: f32) -> f32 {
    size * (rest + (peak - rest) * strength.max(0.0))
}

/// A soft bloom round `galley`, clipped to `area`: copies of the words at
/// growing distance, each ring fainter. The closest a painter gets to the
/// blur of a text-shadow.
fn halo(
    painter: &egui::Painter,
    pos: Pos2,
    galley: &Arc<Galley>,
    area: Rect,
    blur: f32,
    strength: f32,
) {
    let strength = strength.clamp(0.0, 1.0);
    if strength <= 0.0 || blur <= 0.0 {
        return;
    }
    let mut bloom = painter.with_clip_rect(area.intersect(painter.clip_rect()));
    for ring in 1..=GLOW_RINGS {
        let radius = blur * ring as f32 / GLOW_RINGS as f32;
        let fade = 1.0 - (ring - 1) as f32 / GLOW_RINGS as f32;
        bloom.set_opacity(strength * fade * GLOW_COPY);
        for step in 0..GLOW_STEPS {
            let angle = std::f32::consts::TAU * step as f32 / GLOW_STEPS as f32;
            bloom.galley(
                pos + vec2(angle.cos() * radius, angle.sin() * radius),
                galley.clone(),
                Color32::WHITE,
            );
        }
    }
}

/// Spicy Lyrics' glow over a word's own progress: lit at once, held through
/// the word, and gone by its end.
fn word_glow(through: f32) -> f32 {
    curve(through, &[(0.0, 0.0), (0.15, 1.0), (0.6, 1.0), (1.0, 0.0)])
}

/// The same over a line's: up to the middle of the line and back by its end.
fn line_glow(through: f32) -> f32 {
    curve(through, &[(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)])
}

/// The eased line between a curve's points.
fn curve(through: f32, points: &[(f32, f32)]) -> f32 {
    let through = through.clamp(0.0, 1.0);
    for pair in points.windows(2) {
        let ((from, start), (to, end)) = (pair[0], pair[1]);
        if through <= to {
            let along = if to > from {
                (through - from) / (to - from)
            } else {
                0.0
            };
            let eased = along * along * (3.0 - 2.0 * along);
            return start + (end - start) * eased;
        }
    }
    points.last().map_or(0.0, |point| point.1)
}

/// The box round `count` glyphs from glyph `first`, relative to the galley.
fn glyph_area(galley: &Galley, first: usize, count: usize) -> Option<Rect> {
    let mut index = 0;
    let mut area: Option<Rect> = None;
    for row in &galley.rows {
        for glyph in &row.row.glyphs {
            if index >= first && index < first + count {
                let rect = Rect::from_min_size(
                    row.pos + vec2(glyph.pos.x, 0.0),
                    vec2(glyph.advance_width, row.row.size.y),
                );
                area = Some(area.map_or(rect, |area| area.union(rect)));
            }
            index += 1;
        }
    }
    area
}

/// A dot's size and lift as its share of the pause passes: it swells to a
/// little over its size, rises, and settles back, as Spicy Lyrics' dots do.
fn dot_state(fill: f32) -> (f32, f32) {
    (
        curve(fill, &[(0.0, 0.75), (0.7, 1.05), (1.0, 1.0)]),
        curve(fill, &[(0.0, 0.0), (0.9, -0.12), (1.0, 0.0)]),
    )
}

/// Three dots over a long pause, each filling in a third of it, the group
/// breathing gently; `progress` is how much of the pause has passed.
pub(in crate::ui) fn interlude_dots(
    painter: &egui::Painter,
    rect: Rect,
    size: f32,
    progress: f32,
    time: f64,
    opacity: f32,
) {
    let radius = size * 0.16;
    let gap = radius * 3.2;
    // The group breathes while the pause passes.
    let breathe = 1.0 + 0.06 * (time * 2.4).sin() as f32;
    let left = rect.left() + radius;
    for dot in 0..3 {
        let fill = (progress * 3.0 - dot as f32).clamp(0.0, 1.0);
        // Each dot swells as it fills and lifts, then settles: Spicy
        // Lyrics' DotAnimations.
        let (swell, lift) = dot_state(fill);
        let lift = size * lift;
        let at = pos2(left + gap * dot as f32, rect.center().y + lift);
        let dot_radius = radius * breathe * swell;
        // Its glow follows the fill, so the pause lights up as it passes.
        for ring in 1..=3 {
            let spread = dot_radius * (1.0 + 0.55 * ring as f32);
            painter.circle_filled(
                at,
                spread,
                Color32::WHITE.gamma_multiply(0.09 * fill * opacity / ring as f32),
            );
        }
        let alpha = QUIET + (1.0 - QUIET) * fill;
        painter.circle_filled(
            at,
            dot_radius,
            Color32::WHITE.gamma_multiply(alpha * opacity),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(text: &str, at_ms: u32, end_ms: u32) -> Syllable {
        Syllable {
            text: text.into(),
            at_ms,
            end_ms,
        }
    }

    #[test]
    fn pieces_fill_between_their_start_and_end() {
        let pieces = [piece("a", 1_000, 2_000), piece("b", 2_000, 2_000)];
        assert_eq!(progress(&pieces, 500), [0.0, 0.0]);
        assert_eq!(progress(&pieces, 1_500), [0.5, 0.0]);
        assert_eq!(progress(&pieces, 2_000), [1.0, 1.0]);
    }

    /// The light runs through the words in order, across rows, stopping
    /// part way through the word being sung.
    #[test]
    fn the_light_ends_inside_the_word_being_sung() {
        // Two rows: "ab" and "cd", each glyph 10 wide; pieces "ab" and "cd".
        let rows = vec![vec![10.0, 10.0], vec![10.0, 10.0]];
        assert_eq!(
            lit_extents(&rows, &[2, 2], &[1.0, 0.25]),
            Some(vec![20.0, 5.0])
        );
        assert_eq!(
            lit_extents(&rows, &[2, 2], &[0.5, 0.0]),
            Some(vec![10.0, 0.0])
        );
        // Pieces that don't match the glyphs are refused.
        assert_eq!(lit_extents(&rows, &[3], &[1.0]), None);
    }

    /// A word sung out must let the light into the next one: f32 sums of the
    /// glyph widths leave its last glyph a hair short of its width.
    #[test]
    fn a_finished_word_does_not_stop_the_light() {
        // These three advances sum to 44.47625 in f32, which leaves the third
        // 12.4487505 glyph short of its width by a millionth.
        let rows = vec![
            vec![19.880625, 12.146875, 12.4487505],
            vec![10.0, 10.0, 10.0],
        ];
        let extent = lit_extents(&rows, &[3, 3], &[1.0, 0.5]).expect("the glyphs are covered");
        // The first word is lit whole, and the light goes on into the next
        // one, half sung.
        assert!(
            (extent[0] - 44.476_25).abs() < 0.01,
            "the first word is lit whole: {extent:?}"
        );
        assert_eq!(extent[1], 15.0, "the next word is lit: {extent:?}");
    }

    /// Spicy Lyrics' glow: lit at once, held through the word, and gone by
    /// its end; a line's swells to its middle and back.
    #[test]
    fn the_glow_rises_holds_and_falls() {
        assert_eq!(word_glow(0.0), 0.0);
        assert_eq!(word_glow(0.15), 1.0);
        assert_eq!(word_glow(0.4), 1.0);
        assert_eq!(word_glow(1.0), 0.0);
        assert!(word_glow(0.05) > 0.0 && word_glow(0.05) < 1.0);
        assert!(word_glow(0.85) > 0.0 && word_glow(0.85) < 1.0);

        assert_eq!(line_glow(0.0), 0.0);
        assert_eq!(line_glow(0.5), 1.0);
        assert_eq!(line_glow(1.0), 0.0);
        assert!(line_glow(0.25) > 0.0 && line_glow(0.25) < 1.0);
        // Past either end, a line neither glows nor goes negative.
        assert_eq!(line_glow(-0.2), 0.0);
        assert_eq!(line_glow(1.4), 0.0);
    }

    /// A dot waiting is small and still; as its third passes it swells,
    /// rises, and settles, as Spicy Lyrics' dots do.
    #[test]
    fn an_interlude_dot_swells_and_rises() {
        let (waiting, still) = dot_state(0.0);
        assert_eq!((waiting, still), (0.75, 0.0));
        let (filling, up) = dot_state(0.7);
        assert!(
            filling > 1.04 && filling < 1.06,
            "the dot swells: {filling}"
        );
        assert!(up < 0.0, "the dot rises: {up}");
        let (done, settled) = dot_state(1.0);
        assert_eq!((done, settled), (1.0, 0.0));
    }

    /// A space dropped where the line wraps belongs to no glyph.
    #[test]
    fn glyphs_are_matched_to_the_words_they_draw() {
        let pieces = [piece("one ", 0, 1), piece("two", 1, 2)];
        let wrapped: Vec<char> = "onetwo".chars().collect();
        assert_eq!(glyphs_per_piece(&wrapped, &pieces), Some(vec![3, 3]));
        let whole: Vec<char> = "one two".chars().collect();
        assert_eq!(glyphs_per_piece(&whole, &pieces), Some(vec![4, 3]));
        let other: Vec<char> = "one too".chars().collect();
        assert_eq!(glyphs_per_piece(&other, &pieces), None);
    }
}
