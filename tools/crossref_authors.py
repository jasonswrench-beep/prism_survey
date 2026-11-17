"""Full author lists for the Comm Add On references the Directory shortens with "et al." (APA 7 lists every author up to
20). Part of the one-time Crossref lookup (tools/crossref_lookup.py), run after it:

    python tools/crossref_authors.py addons/comm-directory/directory.json addons/comm-directory/doi-lookup.json

Only references already matched by DOI are asked about, by that DOI alone. The authors are added to doi-lookup.json,
and `prismsurvey addon-build` writes them into the reference.
"""

import json
import sys
import time
import urllib.parse
import urllib.request

API = "https://api.crossref.org/works/"
AGENT = "PrismSurvey/0.1 (preparing the Comm Add On references)"


def main():
    directory, lookup = sys.argv[1:3]
    d = json.load(open(directory, encoding="utf-8"))
    lk = json.load(open(lookup, encoding="utf-8"))
    wanted = [
        e for e in d["entries"]
        if "et al." in e.get("source", "")
        and e.get("doi")
        and e["id"] in lk
        and not lk[e["id"]].get("authors")
    ]
    print(f"{len(wanted)} references to complete")
    for e in wanted:
        req = urllib.request.Request(API + urllib.parse.quote(e["doi"]), headers={"User-Agent": AGENT})
        with urllib.request.urlopen(req, timeout=30) as r:
            msg = json.load(r)["message"]
        authors = [
            {"family": a.get("family", ""), "given": a.get("given", ""), "suffix": a.get("suffix")}
            for a in msg.get("author", [])
            if a.get("family")
        ]
        lk[e["id"]]["authors"] = authors
        print(f"  {e['name']}: {len(authors)} authors")
        json.dump(lk, open(lookup, "w", encoding="utf-8", newline="\n"), indent=2, ensure_ascii=False)
        time.sleep(0.5)


if __name__ == "__main__":
    main()
