"""
Parser module - reads raw input documents and converts them into
a structured format the rest of the pipeline can work with.
"""
from validator import validate_document
from storage import save_parsed


SUPPORTED_FORMATS = ["markdown", "plaintext", "json"]


def parse_file(path: str) -> dict:
    """Read a file from disk and return a structured document."""
    with open(path, "r") as f:
        raw = f.read()

    ext = path.rsplit(".", 1)[-1].lower()
    if ext == "md":
        doc = parse_markdown(raw)
    elif ext == "json":
        doc = parse_json(raw)
    else:
        doc = parse_plaintext(raw)

    doc["source"] = path
    return doc


def parse_markdown(text: str) -> dict:
    """Extract title, sections, and links from markdown."""
    lines = text.splitlines()
    title = ""
    sections = []
    links = []

    for line in lines:
        if line.startswith("# ") and not title:
            title = line[2:].strip()
        elif line.startswith("## "):
            sections.append(line[3:].strip())
        elif "](http" in line:
            start = line.index("](") + 2
            end = line.index(")", start)
            links.append(line[start:end])

    return {"title": title, "sections": sections, "links": links, "format": "markdown"}


def parse_json(text: str) -> dict:
    """Parse a JSON document into a structured dict."""
    import json
    data = json.loads(text)
    return {"data": data, "format": "json"}


def parse_plaintext(text: str) -> dict:
    """Split plaintext into paragraphs."""
    paragraphs = [p.strip() for p in text.split("\n\n") if p.strip()]
    return {"paragraph