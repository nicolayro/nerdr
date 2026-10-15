"""Sammenligner nerdr --json med sesjoner.py --json, felt for felt per target."""

import json
import subprocess
import sys

nerdr = json.loads(subprocess.run([sys.argv[1], "--json"], capture_output=True, text=True, check=True).stdout)
python = json.loads(subprocess.run([sys.argv[2], "--json"], capture_output=True, text=True, check=True).stdout)

def per_target(data):
    return {s["target"]: s for s in data["sesjoner"]}

n, p = per_target(nerdr), per_target(python)
print("telling nerdr :", nerdr["telling"])
print("telling python:", python["telling"])
for target in sorted(set(n) | set(p)):
    a, b = n.get(target), p.get(target)
    if a is None or b is None:
        print(f"{target}: bare i {'python' if a is None else 'nerdr'}")
        continue
    ulike = {k: (a.get(k), b[k]) for k in b if k != "aktiv" and a.get(k) != b[k]}
    print(f"{target}: {'likt' if not ulike else ulike}")
