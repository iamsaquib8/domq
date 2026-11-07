#!/usr/bin/env python3
"""Fail when a public name of the JavaScript or Python package is missing from
its reference page in the guide.

    node wasm/npm/build.mjs            # generates wasm/pkg/**/domq_wasm.d.ts
    python3 book/check-api.py

JavaScript: every `export function|class|interface|type` of both `.d.ts`
files (core and extract), the default export (`init`), and every method of
`Document`, must be documented on book/src/javascript.md (the first code span of a
table row, or a code span in a heading). Names that
only the extract entry exports must be in a row under an 'extract' heading or
a row that says "extract".

Python: every name in `__all__` of python/domq/__init__.pyi, and every public
method, property and attribute of each class, must appear in a code span on
book/src/python.md (members inside the section of their class).

Options: --wasm-pkg DIR (default wasm/pkg), --js-page, --py-page, --pyi.
"""
import argparse
import ast
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Declared by wasm-bindgen for every build, not API anyone calls by hand, but
# still listed on the page (the loading table), so they are checked too.
JS_SKIP_MEMBERS = {"constructor", "Symbol.dispose"}


def code_spans(text):
    """The names a page documents: the first code span of every table row and
    every code span in a heading. A name that is only mentioned in running
    text or in an example does not count as documented."""
    spans = re.findall(r"^\|[ ]*`([^`\n]+)`", text, re.M)
    for line in re.findall(r"^#{1,6} .*$", text, re.M):
        spans += re.findall(r"`([^`\n]+)`", line)
    return spans


def has_name(spans, name):
    pattern = re.compile(r"(?<![A-Za-z0-9_$])" + re.escape(name) + r"(?![A-Za-z0-9_$])")
    return any(pattern.search(span) for span in spans)


def headings(text):
    """(level, title, start, end) of every ATX heading's section."""
    found = [(m.start(), len(m.group(1)), m.group(2)) for m in re.finditer(r"^(#{1,6}) +(.+)$", text, re.M)]
    out = []
    for i, (start, level, title) in enumerate(found):
        end = len(text)
        for later_start, later_level, _ in found[i + 1:]:
            if later_level <= level:
                end = later_start
                break
        out.append((level, title, start, end))
    return out


def section_for(text, name):
    """The text of the section whose heading contains `name` in backticks."""
    needle = "`" + name + "`"
    for level, title, start, end in headings(text):
        if needle in title:
            return text[start:end]
    return None


# --- JavaScript --------------------------------------------------------------

def dts_exports(path):
    text = Path(path).read_text()
    names = {}
    for kind, name in re.findall(r"^export (function|class|interface|type) (\w+)", text, re.M):
        names[name] = kind
    if re.search(r"^export default function", text, re.M):
        names["init"] = "function"
    members = []
    m = re.search(r"^export class Document \{\n(.*?)^\}", text, re.M | re.S)
    if m:
        for member in re.findall(r"^    (?:static )?(\w+)\(", m.group(1), re.M):
            members.append(member)
        if "[Symbol.dispose]" in m.group(1):
            members.append("Symbol.dispose")
    return names, members


def check_js(pkg, page_path):
    errors = []
    core_dts = Path(pkg) / "domq_wasm.d.ts"
    extract_dts = Path(pkg) / "extract" / "domq_wasm.d.ts"
    for p in (core_dts, extract_dts):
        if not p.exists():
            return [f"{p} not found: run `node wasm/npm/build.mjs` first"]
    core, core_members = dts_exports(core_dts)
    extract, extract_members = dts_exports(extract_dts)
    page = Path(page_path).read_text()
    document = section_for(page, "Document")
    # Document's methods share names with the functions; do not let one
    # section stand in for the other.
    spans = code_spans(page.replace(document, document.split("\n", 1)[0] + "\n") if document else page)

    for name in sorted(set(core) | set(extract)):
        if not has_name(spans, name):
            errors.append(f"{page_path}: `{name}` ({(core.get(name) or extract.get(name))}) is exported but not on the page")

    only_extract = sorted(set(extract) - set(core))
    marked = []  # spans of table rows that sit in an 'extract' section or say "extract"
    for level, title, start, end in headings(page):
        for line in page[start:end].splitlines():
            m = re.match(r"^\|[ ]*`([^`\n]+)`", line)
            if m and ("extract" in title.lower() or "extract" in line.lower()):
                marked.append(m.group(1))
    for name in only_extract:
        if not has_name(marked, name):
            errors.append(f"{page_path}: `{name}` is only in domq-wasm/extract but its row is not marked as extract")

    if document is None:
        errors.append(f"{page_path}: no section with a `Document` heading")
    else:
        doc_spans = code_spans(document)
        for member in sorted(set(core_members) | set(extract_members)):
            if member in JS_SKIP_MEMBERS:
                needed = {"constructor": "new Document", "Symbol.dispose": "[Symbol.dispose]"}[member]
                if needed not in document:
                    errors.append(f"{page_path}: Document `{needed}` is not documented")
            elif not has_name(doc_spans, member):
                errors.append(f"{page_path}: `Document.{member}` is not documented in the Document section")
    return errors


# --- Python ------------------------------------------------------------------

def pyi_api(path):
    tree = ast.parse(Path(path).read_text())
    all_names = []
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "__all__" for t in node.targets):
            all_names = [elt.value for elt in node.value.elts]
    classes = {}
    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            members = set()
            for item in node.body:
                if isinstance(item, ast.FunctionDef) and not item.name.startswith("_"):
                    members.add(item.name)
                elif isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name) and not item.target.id.startswith("_"):
                    members.add(item.target.id)
            classes[node.name] = members
    return all_names, classes


def check_py(pyi, page_path):
    errors = []
    all_names, classes = pyi_api(pyi)
    if not all_names:
        return [f"{pyi}: no __all__ found"]
    page = Path(page_path).read_text()
    spans = code_spans(page)
    for name in all_names:
        if not has_name(spans, name):
            errors.append(f"{page_path}: `{name}` is in __all__ but not on the page")
    for cls, members in sorted(classes.items()):
        if cls not in all_names:
            continue
        section = section_for(page, cls)
        if section is None:
            errors.append(f"{page_path}: no section with a `{cls}` heading")
            continue
        section_spans = code_spans(section)
        for member in sorted(members):
            if not has_name(section_spans, member):
                errors.append(f"{page_path}: `{cls}.{member}` is not documented in the `{cls}` section")
    return errors


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--wasm-pkg", default=ROOT / "wasm" / "pkg")
    ap.add_argument("--js-page", default=ROOT / "book" / "src" / "javascript.md")
    ap.add_argument("--py-page", default=ROOT / "book" / "src" / "python.md")
    ap.add_argument("--pyi", default=ROOT / "python" / "domq" / "__init__.pyi")
    args = ap.parse_args()

    errors = check_js(args.wasm_pkg, args.js_page) + check_py(args.pyi, args.py_page)
    for line in errors:
        print(line, file=sys.stderr)
    if errors:
        print(f"\n{len(errors)} public name(s) missing from the reference pages.", file=sys.stderr)
        return 1
    print("reference pages cover every exported name")
    return 0


if __name__ == "__main__":
    sys.exit(main())
