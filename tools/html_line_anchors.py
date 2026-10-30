#!/usr/bin/env python3
"""Compile Typst HTML with source-line anchors using Tree-sitter."""
import re
from functools import lru_cache

# TODO: Once a released py-tree-sitter contains the Point getter refcount fix,
# verify the Windows wheel with long Typst documents before reconsidering .row.
# 0.26.0's .row/.column return borrowed references; tuple indexing avoids them.
# https://github.com/tree-sitter/py-tree-sitter/issues/487
# https://github.com/tree-sitter/py-tree-sitter/blob/master/tree_sitter/binding/point.c


@lru_cache(maxsize=1)
def typst_parser():
    from tree_sitter_language_pack import get_parser
    return get_parser("typst")


def check_document_settings(source):
    """Explain the template's top-level document metadata requirement."""
    try:
        data = source.encode("utf-8")
        tree = typst_parser().parse(data)
    except Exception as error:
        raise ValueError(f"could not load the Typst grammar: {error}") from error
    if tree.root_node.has_error:
        return
    template_seen = False
    for node in tree.root_node.named_children:
        if node.type != "code" or len(node.named_children) != 1:
            continue
        child = node.named_children[0]
        text = data[child.start_byte:child.end_byte]
        if child.type == "show" and re.match(rb"show\s*:\s*template\b", text):
            template_seen = True
        elif template_seen and child.type == "set" and re.match(rb"set\s+document\s*\(", text):
            line = node.start_point[0] + 1
            raise ValueError(f"line {line}: #set document must appear before #show: template; move the document setting above the template show rule")


def inject(source):
    try:
        parser = typst_parser()
    except ImportError as error:
        raise ValueError("install dependencies: python -m pip install -r requirements.txt") from error
    data = source.encode("utf-8")
    try:
        tree = parser.parse(data)
    except Exception as error:
        raise ValueError(f"could not load the Typst grammar: {error}") from error
    if tree.root_node.has_error:
        raise ValueError("Typst syntax tree contains errors; refusing to inject markers")
    anchors = {}
    labels = set(re.findall(r"<L([1-9]\d*)>", source))

    def mark(node, heading=False):
        line = node.start_point[0] + 1
        if str(line) in labels:
            raise ValueError(f"line anchor L{line} conflicts with a source label")
        if line in anchors:
            return
        offset = node.start_byte
        if heading:
            prefix = re.match(rb'=+[ \t]+', data[offset:node.end_byte])
            if not prefix:
                return
            offset += prefix.end()
        marker = (f'#html.elem("span", attrs: (id: "L{line}", '
                  f'"data-line": "{line}", class: "source-line-anchor"))')
        anchors[line] = (offset, marker.encode())

    def visit(container):
        paragraph_start = True
        for node in container.named_children:
            if node.type == "section":
                visit(node)
                paragraph_start = True
            elif node.type == "heading":
                mark(node, heading=True)
                paragraph_start = True
            elif node.type == "content":
                visit(node)
                paragraph_start = True
            elif node.type == "parbreak":
                paragraph_start = True
            elif node.type == "comment":
                continue
            else:
                if paragraph_start and node.type == "text" and data[node.start_byte:node.end_byte].strip():
                    mark(node)
                paragraph_start = False

    visit(tree.root_node)
    chunks = []
    previous = 0
    for offset, marker in sorted(anchors.values()):
        chunks.extend((data[previous:offset], marker))
        previous = offset
    chunks.append(data[previous:])
    return b"".join(chunks).decode("utf-8")
