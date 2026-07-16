from pathlib import Path
import re
import subprocess
from typing import Optional


_INCLUDE_RE = re.compile(r'INCLUDE!\("([^"]*)"\)')


def get_cargo_workspace_root() -> Path:
    """Get the Cargo workspace root directory."""
    cargo_toml = subprocess.check_output(
        ["cargo", "locate-project", "--workspace", "--message-format", "plain"],
        text=True,
    ).strip()

    return Path(cargo_toml).parent


def resolve_includes(
    input_file: Path,
    output_file: Path,
    shorthands: Optional[dict[str, str]] = None,
) -> None:
    print(f"resolve_includes({input_file=}, {output_file=})")

    if shorthands is None:
        shorthands = {}

    workspace_root = None
    output_lines = []

    text = input_file.read_text()

    for line in text.splitlines():
        match = _INCLUDE_RE.search(line)

        if not match:
            output_lines.append(line)
            continue

        include_path_str = match.group(1)

        for old, new in shorthands.items():
            include_path_str = include_path_str.replace(old, new)

        if include_path_str.startswith("/"):
            if workspace_root is None:
                workspace_root = get_cargo_workspace_root()
            include_path = workspace_root / include_path_str.lstrip("/")
        else:
            include_path = input_file.parent / include_path_str

        print(f"Resolving INCLUDE!(\"{match.group(1)}\") -> {include_path}")

        included_text = include_path.read_text()

        # Determine indentation from characters before INCLUDE!
        prefix = line[: match.start()]
        if all(c == " " for c in prefix):
            indent_spaces = len(prefix)
            indent = " " * indent_spaces

            included_lines = [l for l in included_text.splitlines() if "INCLUDE_IGNORE_LINE" not in l]
            
            # strip common indent
            included_lines_common_indent = min(
                (
                    len(l) - len(l.lstrip())
                    for l in included_lines
                    if l and not l.isspace()
                ),
                default=0
            )
            if included_lines_common_indent > 0:
                included_lines = [l.removeprefix(" " * included_lines_common_indent) for l in included_lines]

            included_text = "\n".join(
                (indent + l if l.strip() else l)
                for l in included_lines
            )

        # Preserve anything after INCLUDE!() on the same line
        suffix = line[match.end():]
        output_lines.append(included_text)
        if suffix:
            output_lines.append(suffix)

    output_file.write_text("\n".join(output_lines))
