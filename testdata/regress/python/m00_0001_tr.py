"""Tests for the Terraform/HCL extractor (graphify/extract.py, issue #187)."""
from __future__ import annotations

from pathlib import Path

from graphify.build import build_from_json
from graphify.extract import extract_terraform


def _write(tmp_path: Path, name: str, body: str) -> Path:
    p = tmp_path / name
    p.write_text(body, encoding="utf-8")
    return p


def _labels(r) -> list[str]:
    return [n["label"] for n in r["nodes"]]


def _rel_pairs(r, relation: str) -> set[tuple[str, str]]:
    lab = {n["id"]: n["label"] for n in r["nodes"]}
    return {
        (lab.get(e["source"], e["source"]), lab.get(e["target"], e["target"]))
        for e in r["edges"]
        if e["relation"] == relation
    }


SAMPLE = """\
# leading comment so the body is not children[0]
terraform {
  required_providers { azurerm = { source = "hashicorp/azurerm" } }
}

variab