#!/bin/sh
# Re-scrape the public project directory into ./out/projects.json, then fetch demo-post text.
# Usage: sh research/scripts/fetch_projects.sh
set -e
OUT=out; mkdir -p "$OUT/pages"
TOTAL=$(curl -fsSL "https://nearhackathon.com/api/projects?page=1" | python3 -c 'import json,sys,math;d=json.load(sys.stdin);print(math.ceil(d["total"]/d["pageSize"]))')
for i in $(seq 1 "$TOTAL"); do curl -fsSL "https://nearhackathon.com/api/projects?page=$i" -o "$OUT/pages/$i.json"; sleep 0.3; done
python3 -c 'import json,glob;P={p["id"]:p for f in glob.glob("'"$OUT"'/pages/*.json") for p in json.load(open(f))["projects"]};json.dump(list(P.values()),open("'"$OUT"'/projects.json","w"),indent=1);print(len(P),"projects")'
python3 "$(dirname "$0")/fetch_tweets.py" "$OUT"
