"""Writes the review list for the Comm Add On's references after the Crossref lookup:

    python tools/reference_review.py addons/comm-directory/directory.json addons/comm-directory/doi-lookup.json addons/comm-directory/REFERENCE-REVIEW.md

Close calls (Crossref's best match differs a little from the book) and references with no match are listed with the
book's text and Crossref's suggestion, so a person can accept, correct or leave each one. Accepted matches are
summarised only.
"""

import json
import sys


def main():
    directory, lookup, out = sys.argv[1:4]
    d = json.load(open(directory, encoding="utf-8"))
    lk = json.load(open(lookup, encoding="utf-8"))
    by_id = {e["id"]: e for e in d["entries"]}
    review = [(i, v) for i, v in lk.items() if v["status"] == "review" and (by_id.get(i, {}).get("referenceCheck") or "").startswith("Check: Crossref's closest")]
    accepted = [i for i, v in lk.items() if v["status"] == "review" and "first page" in (by_id.get(i, {}).get("referenceCheck") or "")]
    missing = [(i, v) for i, v in lk.items() if v["status"] == "no match"]
    matched = [i for i, v in lk.items() if v["status"] == "matched"] + accepted
    added = sum(1 for e in d["entries"] if (e.get("referenceCheck") or "").startswith("DOI and issue number"))
    already = sum(1 for e in d["entries"] if "already gives the issue" in (e.get("referenceCheck") or ""))
    flagged = [e for e in d["entries"] if (lk.get(e["id"], {}).get("status") == "matched" or e["id"] in accepted) and (e.get("referenceCheck") or "").startswith("Check")]
    et_al = [e for e in d["entries"] if "et al." in e.get("source", "")]
    lines = [
        "# Comm Add On: references to review",
        "",
        "From the one-time Crossref lookup (tools/crossref_lookup.py). APA 7 asks for the issue number and the DOI, which the",
        "Directory (written for APA 6) does not give.",
        "",
        f"- {len(matched)} references matched ({len(accepted)} of them close calls accepted because the volume and first page agree with the book): their DOI is added; the issue number is added for {added} and was already in the book for {already}.",
        f"- {len(flagged)} matched references with something to check (a volume that differs from Crossref's, or an author list cut short with et al.), listed below.",
        f"- {len(review)} close calls, listed below: nothing was changed; accept or correct each by hand.",
        f"- {len(missing)} with no match, listed below: often books, chapters, conference papers, or journals without DOIs.",
        "",
        "## Matched, with something to check",
        "",
    ]
    for e in sorted(flagged, key=lambda e: e["name"]):
        lines += [f"- **{e['name']}**: {e['sourceApa7']}", f"  - {e['referenceCheck']}"]
    lines += [
        "",
        "## Close calls",
        "",
    ]
    for i, v in sorted(review, key=lambda x: by_id.get(x[0], {}).get("name", "")):
        e = by_id.get(i, {})
        lines += [
            f"### {e.get('name', i)}",
            "",
            f"- Book: {v.get('source', '')}",
            f"- Crossref: {v.get('crossrefTitle', '')} ({v.get('crossrefYear', '')}), {v.get('journal', '')}, {v.get('volume') or ''}({v.get('issue') or ''}), {v.get('pages') or ''}, DOI {v.get('doi', '')}",
            f"- Title words in common: {v.get('titleSimilarity', 0):.0%}",
            "",
        ]
    lines += ["## No match", ""]
    for i, v in sorted(missing, key=lambda x: by_id.get(x[0], {}).get("name", "")):
        e = by_id.get(i, {})
        lines.append(f"- **{e.get('name', i)}**: {v.get('source', '')}")
        if "et al." in v.get("source", ""):
            lines.append("  - The book shortens the author list with et al.; APA 7 lists every author (up to 20).")
    open(out, "w", encoding="utf-8", newline="\n").write("\n".join(lines) + "\n")
    print(f"{out}: {len(review)} close calls, {len(missing)} with no match, {len(matched)} matched")


if __name__ == "__main__":
    main()
