#!/usr/bin/env python3
"""Adds what is new in mcloud to MCLOUD-GOREVLER.md's inbox.

mcloud (the Spicetify theme mcloudfast ports) keeps its comparisons with Apple
Music in STATUS.md, and lands its work as pull requests that close issues.
This lists, since the inbox's `since` mark: the pull requests merged into
mcloud's main, the issues closed as completed, and the commits pushed to main
without a pull request that changed STATUS.md. Each becomes one line in the
inbox, unless its link is already in the file; the mark moves to an hour
before now, so items GitHub's search indexes late are still found next time
and the link check drops the repeats.

Usage: mcloud-sync.py [--file MCLOUD-GOREVLER.md] [--repo mcloudHQ/mcloud]
                      [--now 2026-11-04T06:00:00Z] [--dry-run]
Reads mcloud through `gh` (GH_TOKEN); mcloud is public, so the workflow's own
token is enough. Writes `added=<count>` to $GITHUB_OUTPUT and the new lines to
$GITHUB_STEP_SUMMARY when they are set.
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys

START = re.compile(r"<!-- mcloud-sync:start since=(\S+) -->")
END = "<!-- mcloud-sync:end -->"
THEME = ("theme.js", "user.css")
OVERLAP = datetime.timedelta(hours=1)


def parse_time(text):
    return datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))


def format_time(moment):
    return moment.astimezone(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def gh(*args):
    result = subprocess.run(["gh", *args], check=True, capture_output=True, text=True)
    return json.loads(result.stdout or "null")


def pull_requests(repo, since):
    found = gh(
        "pr", "list", "-R", repo, "--state", "merged", "--base", "main", "--limit", "100",
        "--search", f"merged:>={format_time(since)}",
        "--json", "number,title,url,mergedAt,files",
    )
    items = []
    for pr in found:
        files = [entry["path"] for entry in pr.get("files") or []]
        notes = []
        if "STATUS.md" in files:
            notes.append("STATUS.md değişti")
        if not any(path in THEME for path in files):
            notes.append("tema dosyası yok")
        items.append(item(pr["mergedAt"], f"PR #{pr['number']}", pr["title"], pr["url"], notes))
    return items


def issues(repo, since):
    found = gh(
        "issue", "list", "-R", repo, "--state", "closed", "--limit", "100",
        "--search", f"closed:>={format_time(since)} reason:completed",
        "--json", "number,title,url,closedAt,stateReason",
    )
    return [
        item(issue["closedAt"], f"Issue #{issue['number']}", issue["title"], issue["url"], [])
        for issue in found
        if issue.get("stateReason") == "COMPLETED"
    ]


def status_commits(repo, since):
    """Commits on main that changed STATUS.md outside any pull request."""
    found = gh("api", f"repos/{repo}/commits?sha=main&path=STATUS.md&since={format_time(since)}&per_page=100")
    items = []
    for commit in found:
        if gh("api", f"repos/{repo}/commits/{commit['sha']}/pulls"):
            continue
        title = commit["commit"]["message"].splitlines()[0]
        items.append(item(commit["commit"]["committer"]["date"], f"Commit {commit['sha'][:7]}", title, commit["html_url"], ["STATUS.md değişti"]))
    return items


def item(when, kind, title, url, notes):
    return {"when": when, "kind": kind, "title": title, "url": url, "notes": notes}


def line(entry):
    notes = f" ({', '.join(entry['notes'])})" if entry["notes"] else ""
    title = entry["title"].replace("[", "\\[").replace("]", "\\]")
    return f"- {entry['when'][:10]} · [{entry['kind']}]({entry['url']}): {title}{notes}"


def update(text, entries, since, now):
    """The file with the new entries added to the inbox and the mark moved.

    Entries at or before `since`, and those whose link the file already
    holds, are left out. Returns the new text and the lines added.
    """
    start = START.search(text)
    end = text.find(END)
    if not start or end < start.end():
        raise SystemExit(f"the inbox marks ({START.pattern} … {END}) are missing")
    fresh = sorted(
        (entry for entry in entries if parse_time(entry["when"]) > since and entry["url"] not in text),
        key=lambda entry: (entry["when"], entry["kind"]),
    )
    added = []
    for entry in fresh:
        if entry["url"] not in {e["url"] for e in added}:
            added.append(entry)
    lines = [line(entry) for entry in added]
    mark = max(since, now - OVERLAP)
    body = text[start.end():end].strip("\n")
    inbox = "\n".join(part for part in (body, *lines) if part)
    head = f"<!-- mcloud-sync:start since={format_time(mark)} -->"
    return text[:start.start()] + head + "\n" + (inbox + "\n" if inbox else "") + text[end:], lines


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--file", default="MCLOUD-GOREVLER.md")
    parser.add_argument("--repo", default="mcloudHQ/mcloud")
    parser.add_argument("--now", help="the current time, for tests")
    parser.add_argument("--dry-run", action="store_true", help="print the new lines, change nothing")
    args = parser.parse_args()

    now = parse_time(args.now) if args.now else datetime.datetime.now(datetime.timezone.utc)
    with open(args.file, encoding="utf-8") as handle:
        text = handle.read()
    start = START.search(text)
    if not start:
        raise SystemExit(f"{args.file}: no `{START.pattern}` mark")
    since = parse_time(start.group(1))
    query = since - OVERLAP
    entries = pull_requests(args.repo, query) + issues(args.repo, query) + status_commits(args.repo, query)
    new_text, lines = update(text, entries, since, now)

    print("\n".join(lines) if lines else "Nothing new in mcloud.")
    if not args.dry_run and new_text != text:
        with open(args.file, "w", encoding="utf-8") as handle:
            handle.write(new_text)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as handle:
            handle.write(f"added={len(lines)}\n")
    if lines and os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write("New in mcloud:\n\n" + "\n".join(lines) + "\n")


if __name__ == "__main__":
    sys.exit(main())
