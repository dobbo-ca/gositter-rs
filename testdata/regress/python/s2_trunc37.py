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

    for line 