#!/usr/bin/env python3
"""Regenerate the seed corpora from the sources "Seeds and dictionary" names.

Run from the repository root:

    python3 fuzz/make_seeds.py

Writes `fuzz/seeds/convert` and `fuzz/seeds/round_trip`; the structured target
reads its bytes as generator choices and gets no HTML seeds. Every file starts
with the 13-byte option prefix: all zero except a 1 for `translation_mode`, so
each target starts from Faithful mode with the default options.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PREFIX = b"\x00" * 11 + b"\x01" + b"\x00"

SEED_TABLE = re.compile(r"^\|.*\|\s*$")


def slug(heading: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", heading.lower()).strip("-")


def translation_approach_rows():
    """Yield (table number, slug, row number, variant, HTML) for every
    `HTML in` cell of every table."""
    lines = (ROOT / "translation_approach.md").read_text(encoding="utf-8").splitlines()
    section = ""
    table_number = 0
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("### "):
            section = line[4:].strip()
        if SEED_TABLE.match(line) and "HTML in" in line:
            table_number += 1
            columns = [cell.strip() for cell in line.strip("|").split("|")]
            html_column = columns.index("HTML in")
            # The separator row follows the header.
            row_number = 0
            index += 2
            while index < len(lines) and SEED_TABLE.match(lines[index]):
                row_number += 1
                cells = [cell.strip() for cell in lines[index].strip("|").split("|")]
                html = cells[html_column]
                # The cells are written as Markdown inline code.
                if html.startswith("`") and html.endswith("`"):
                    html = html[1:-1]
                html = html.replace("␣", " ").replace("⏎", "\n")
                marker = "<br><br>...<br>"
                if marker in html:
                    variants = [html.replace(marker, "<br><br>"), html.replace(marker, "<br><br><br>")]
                else:
                    variants = [html]
                if section == "Tables":
                    variants = [
                        "<table><thead><tr><th>h</th></tr></thead>"
                        f"<tbody><tr>{variant}</tr></tbody></table>"
                        for variant in variants
                    ]
                for variant, expanded in enumerate(variants, start=1):
                    yield table_number, slug(section), row_number, variant, expanded
                index += 1
            continue
        index += 1


def turndown_cases():
    """The `<div class="input">` payload of each case of the turndown index."""
    source = (ROOT / "tests/turndown_cases/turndown_test_index.html").read_text(encoding="utf-8")
    cases = []
    position = 0
    while True:
        start = source.find('<div class="input">', position)
        if start == -1:
            break
        cursor = start + len('<div class="input">')
        depth = 1
        for match in re.finditer(r"<div\b[^>]*>|</div>", source[cursor:]):
            if match.group(0).startswith("</"):
                depth -= 1
            else:
                depth += 1
            if depth == 0:
                cases.append(source[cursor : cursor + match.start()])
                position = cursor + match.end()
                break
        else:
            raise ValueError("unclosed input div")
    return cases


def deep_nesting(open_tag: str, close_tag: str) -> bytes:
    """The deepest repetition of `open_tag` which fits in 64 KiB, prefix included."""
    word = b"word"
    unit = (open_tag + close_tag).encode("utf-8")
    depth = (65536 - len(PREFIX) - len(word)) // len(unit)
    return open_tag.encode("utf-8") * depth + word + close_tag.encode("utf-8") * depth


def write_seed(directory: Path, name: str, html: str | bytes) -> None:
    data = PREFIX + (html.encode("utf-8") if isinstance(html, str) else html)
    (directory / name).write_bytes(data)


def main() -> None:
    for target in ("convert", "round_trip"):
        directory = ROOT / "fuzz" / "seeds" / target
        directory.mkdir(parents=True, exist_ok=True)
        for stale in directory.iterdir():
            stale.unlink()
        for table, name, row, variant, html in translation_approach_rows():
            write_seed(
                directory,
                f"t{table:02d}-{name}-r{row:02d}-v{variant}.html",
                html,
            )
        for number, case in enumerate(turndown_cases(), start=1):
            write_seed(directory, f"turndown-{number:03d}.html", case)

    convert = ROOT / "fuzz" / "seeds" / "convert"
    hacker_news = (ROOT / "examples/page-to-markdown/html/Hacker News.html").read_bytes()
    write_seed(convert, "hacker-news.html", hacker_news)
    for name, (open_tag, close_tag) in {
        "deep-blockquote": ("<blockquote>", "</blockquote>"),
        "deep-ul-li": ("<ul><li>", "</li></ul>"),
        "deep-em": ("<em>", "</em>"),
        "deep-table-tr-td": ("<table><tr><td>", "</td></tr></table>"),
        "deep-span": ("<span>", "</span>"),
    }.items():
        write_seed(convert, f"{name}.html", deep_nesting(open_tag, close_tag))


if __name__ == "__main__":
    main()
