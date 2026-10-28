#!/usr/bin/env python3
"""Create a NEW synthetic container; never inspect a real Steam installation."""
import argparse
import json
from pathlib import Path
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("destination", type=Path)
args = parser.parse_args()
root = args.destination.absolute()
if any(ord(c) < 32 for c in str(root)):
    parser.error("fixture destination must not contain control characters")
root.mkdir(exist_ok=False)  # Refuse existing data, including symlinks.
(root / ".koine-synthetic-root").write_text(f"koine-fixture-v1\n{uuid.uuid4()}\n")
(root / "steamapps").mkdir()
extra = root / "library B"
(extra / "steamapps").mkdir(parents=True)
# Each root can also be selected independently by controlled discovery v2.
(extra / ".koine-synthetic-root").write_text(f"koine-fixture-v1\n{uuid.uuid4()}\n")
(extra / "steamapps/libraryfolders.vdf").write_text(
    '"libraryfolders" { "0" { "path" ' + json.dumps(str(extra), ensure_ascii=False) + ' } }\n'
)
# JSON quoting matches the supported quoted KeyValues escapes for these paths.
index = '"libraryfolders" {\n'
for i, path in enumerate([root, extra]):
    index += f'"{i}" {{ "path" {json.dumps(str(path), ensure_ascii=False)} "apps" {{}} }}\n'
(root / "steamapps/libraryfolders.vdf").write_text(index + "}\n")
for library, app, flags in [(root, 900001, 4), (extra, 900002, 1048576)]:
    (library / f"steamapps/appmanifest_{app}.acf").write_text(
        f'"AppState" {{ "appid" "{app}" "name" "Synthetic {app}" '
        f'"installdir" "Synthetic {app}" "StateFlags" "{flags}" }}\n'
    )
print(root)
