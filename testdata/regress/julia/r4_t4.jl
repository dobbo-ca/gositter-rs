# This file was copied from Transducers.jl
# which is available under an MIT license (see LICENSE).
using PkgBenchmark
using Markdown

function displayresult(result)
    md = sprint(export_markdown, result)
    md = replace(md, ":x:" => "❌")
  