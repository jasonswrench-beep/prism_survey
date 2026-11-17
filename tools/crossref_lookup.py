"""One-time lookup of DOIs, volumes, issues and pages for the Comm Add On's references (APA 7 asks for the issue
number and the DOI, which the Directory, written for APA 6, does not give).

Run once when the add-on is prepared, with Jason's go-ahead (given 2026-11-17):

    python tools/crossref_lookup.py addons/comm-directory/directory.json addons/comm-directory/doi-lookup.json

For each entry's source it asks Crossref's public API for the best bibliographic match and keeps it only when the
title and year agree. Nothing about the person is sent: only the reference text, with a plain program name. Results go
to doi-lookup.json (the run can be stopped and started again; finished entries are kept), and `prismsurvey
addon-build` merges the accepted matches into directory.json. Close calls are marked for review, never accepted.
"""

import json
import re
import sys
import time
import urllib.parse
import urllib.request

API = "https://api.crossref.org/works"
AGENT = "PrismSurvey/0.1 (preparing the Comm Add On references)"


def words(s):
    return [w for w in re.findall(r"[a-z0-9]+", s.lower()) if len(w) > 2]


def similarity(a, b):
    sa, sb = set(words(a)), set(words(b))
    if not sa or not sb:
        return 0.0
    return len(sa & sb) / len(sa | sb)


def parts(ref):
    """Year and title from an APA reference: 'Authors (1984). Title. Journal, 33, 1-2.'"""
    m = re.search(r"\((\d{4})[a-z]?\)\.\s*(.+)", ref)
    if not m:
        return None, ""
    year = int(m.group(1))
    rest = m.group(2)
    # The title runs to the first full stop followed by a capital letter (the journal).
    t = re.match(r"(.+?[.?!])\s+[A-Z]", rest)
    title = (t.group(1) if t else rest).rstrip(".")
    return year, title


def first_author(ref):
    m = re.match(r"\s*([A-Za-z'\-]+)", ref)
    return m.group(1).lower() if m else ""


def ask(ref):
    q = urllib.parse.urlencode({"query.bibliographic": ref, "rows": 3, "select": "DOI,title,issued,container-title,volume,issue,page,author,type"})
    req = urllib.request.Request(f"{API}?{q}", headers={"User-Agent": AGENT})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)["message"]["items"]


def judge(ref, item):
    year, title = parts(ref)
    got_title = (item.get("title") or [""])[0]
    got_year = None
    for k in ("issued",):
        dp = (item.get(k) or {}).get("date-parts") or [[None]]
        got_year = dp[0][0]
    sim = similarity(title, got_title) if title else 0.0
    authors = [a.get("family", "").lower() for a in item.get("author", [])]
    author_ok = first_author(ref) in authors if authors else False
    year_ok = year is not None and got_year is not None and abs(year - got_year) <= 1
    if sim >= 0.8 and year_ok and (author_ok or sim >= 0.95):
        status = "matched"
    elif sim >= 0.5 and year_ok:
        status = "review"
    else:
        status = "no match"
    return {
        "status": status,
        "doi": item.get("DOI"),
        "volume": item.get("volume"),
        "issue": item.get("issue"),
        "pages": item.get("page"),
        "crossrefTitle": got_title,
        "crossrefYear": got_year,
        "journal": (item.get("container-title") or [""])[0],
        "titleSimilarity": round(sim, 2),
        "authorMatches": author_ok,
    }


def main():
    directory, out = sys.argv[1], sys.argv[2]
    d = json.load(open(directory, encoding="utf-8"))
    try:
        done = json.load(open(out, encoding="utf-8"))
    except FileNotFoundError:
        done = {}
    todo = [e for e in d["entries"] if e["id"] not in done]
    print(f"{len(d['entries'])} entries, {len(todo)} still to look up")
    for k, e in enumerate(todo, 1):
        ref = e["source"]
        result = {"status": "no match", "source": ref}
        try:
            items = ask(ref)
            best = None
            for item in items:
                j = judge(ref, item)
                rank = {"matched": 2, "review": 1, "no match": 0}[j["status"]]
                if best is None or (rank, j["titleSimilarity"]) > best[0]:
                    best = ((rank, j["titleSimilarity"]), j)
            if best:
                result = {**best[1], "source": ref}
        except Exception as ex:  # A failed request is recorded, and tried again on the next run.
            print(f"  {e['id']}: {ex}")
            time.sleep(5)
            continue
        done[e["id"]] = result
        if k % 20 == 0 or k == len(todo):
            json.dump(done, open(out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
            counts = {s: sum(1 for v in done.values() if v["status"] == s) for s in ("matched", "review", "no match")}
            print(f"  {len(done)} done: {counts}")
        time.sleep(0.25)
    json.dump(done, open(out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)


if __name__ == "__main__":
    main()
