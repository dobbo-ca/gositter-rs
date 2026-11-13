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
    return {"paragraphs": paragraphs, "format": "plaintext"}


def parse_and_save(path: str) -> str:
    """Full pipeline: parse, validate, save. Returns the saved record ID."""
    doc = parse_file(path)
    validated = validate_document(doc)
    record_id = save_parsed(validated)
    return record_id


def batch_parse(paths: list) -> list:
    """Parse a list of files and return their record IDs."""
    results = []
    for path in paths:
        try:
            rid = parse_and_save(path)
            results.append({"path": path, "id": rid, "ok": True})
        except Exception as e:
            results.append({"path": path, "error": str(e), "ok": False})
    return results
