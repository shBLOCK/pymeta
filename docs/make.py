import subprocess
import sys
from pathlib import Path

DOCS_ROOT = Path(".")
MISC_ROOT = DOCS_ROOT / "misc"

print("Generating theme/highlight.js", flush=True)
with open(DOCS_ROOT / "theme/highlight.js", "w") as highlight_js_file:
    highlight_js_file.write(open(MISC_ROOT / "hljs/highlight.min.js").read())
    highlight_js_file.write("\n" * 3)
    highlight_js_file.write(open(MISC_ROOT / "highlight_addon.js").read())

cmd = ["mdbook", *sys.argv[1:]]
print(" ".join(cmd), flush=True)
exit(subprocess.run(cmd).returncode)