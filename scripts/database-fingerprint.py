"""Read only fingerprints for checking installer upgrades without exposing task content."""
import hashlib
import json
import sqlite3
import sys
from pathlib import Path

with sqlite3.connect(Path(sys.argv[1]).resolve().as_uri() + "?mode=ro", uri=True) as database:
    rows = {table: [row[0] for row in database.execute(f"SELECT payload FROM {table} ORDER BY payload")]
            for table in ("tasks", "series", "settings")}
print(json.dumps({"task_count": len(rows["tasks"]), "series_count": len(rows["series"]),
                  "sha256": hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()}))
