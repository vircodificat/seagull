#!/usr/bin/env python3
"""
Validate theory markdown files for steno outline consistency.

This script:
1. Iterates through all .md files in the theory/ directory
2. Parses markdown links of the form [word](outline)
3. Tracks word, outline, and source file
4. Detects conflicts where the same word maps to different outlines
5. Outputs theory.json if no conflicts are found
"""

import seagull
import json
import re
import sys
from pathlib import Path
from collections import defaultdict


# Root directory is one level up from this script
SCRIPT_DIR = Path(__file__).parent
ROOT_DIR = SCRIPT_DIR.parent
THEORY_DIR = ROOT_DIR / 'theory'
BUILD_DIR = ROOT_DIR / 'build'


def parse_markdown_links(content, filename):
    """
    Parse markdown links from content.

    Looks for patterns like [word](outline) and returns list of tuples.
    Only matches links that look like steno outlines (alphanumeric/dash patterns).
    """
    links = []
    # Pattern matches [word](outline) format
    pattern = r'\[([^\]]+)\]\(([^)]+)\)'

    for match in re.finditer(pattern, content):
        word = match.group(1)
        outline = match.group(2)

        # Only include if outline looks like a steno outline
        # (contains letters, numbers, hyphens, slashes, asterisks)
        if re.match(r'^[A-Za-z0-9\-/*]*$', outline) and outline:
            links.append({
                'word': word,
                'outline': outline,
                'source': filename
            })

    return links


def find_md_files(theory_dir=THEORY_DIR):
    """Find all .md files in the theory directory."""
    return sorted(theory_dir.glob('*.md'))


def validate_theory(theory_dir=THEORY_DIR):
    """
    Validate theory markdown files.

    Returns tuple of (all_links, missings, invalids, conflicts) where:
    - all_links: list of all parsed links
    - missings: dict mapping words to missing outlines
    - invalids: dict mapping words to invalid outlines
    - conflicts: dict mapping words to conflicting outlines
    """
    md_files = find_md_files(theory_dir)

    if not md_files:
        print(f"No .md files found in {theory_dir}")
        return [], {}, {}, {}

    all_links = []
    missing_links = []
    word_to_outlines = defaultdict(set)
    word_to_sources = defaultdict(set)

    # Parse all markdown files
    for md_file in md_files:
        print(f"Parsing {md_file.name}...", file=sys.stderr)
        content = md_file.read_text(encoding='utf-8')
        links = parse_markdown_links(content, md_file.name)

        # Also find missing outlines [word]() format
        pattern = r'\[([^\]]+)\]\(\s*\)'
        for match in re.finditer(pattern, content):
            word = match.group(1)
            missing_links.append({
                'word': word,
                'source': md_file.name
            })

        for link in links:
            all_links.append(link)
            word_to_outlines[link['word']].add(link['outline'])
            word_to_sources[link['word']].add(link['source'])

    # Find Missing Outlines
    missings = {}
    for missing_link in missing_links:
        word = missing_link['word']
        if word not in missings:
            missings[word] = {
                'sources': set()
            }
        missings[word]['sources'].add(missing_link['source'])

    # Convert sets to sorted lists for missings
    for word in missings:
        missings[word]['sources'] = sorted(list(missings[word]['sources']))

    # Find Invalid Outlines
    invalids = {}
    for word, outlines in word_to_outlines.items():
        for outline in outlines:
            if seagull.Outline.from_extended(outline) is None:
                invalids[word] = {
                    'outlines': sorted(list(outlines)),
                    'sources': sorted(list(word_to_sources[word]))
                }
                break

    # Find conflicts
    conflicts = {}
    for word, outlines in word_to_outlines.items():
        if len(outlines) > 1:
            conflicts[word] = {
                'outlines': sorted(list(outlines)),
                'sources': sorted(list(word_to_sources[word]))
            }

    return all_links, missings, invalids, conflicts


def output_results(all_links, missings, invalids, conflicts, build_dir=BUILD_DIR, theory_dir=THEORY_DIR):
    """Output results to console and file if no conflicts."""
    if invalids:
        print("\n❌ INVALIDS FOUND:", file=sys.stderr)
        print(f"Found {len(invalids)} word(s) with invalid outlines:\n", file=sys.stderr)
        for word in sorted(invalids.keys()):
            invalid_info = invalids[word]
            print(f"  Word: '{word}'", file=sys.stderr)
            for outline in invalid_info['outlines']:
                # Find sources for this word-outline pair
                for link in all_links:
                    if link['word'] == word and link['outline'] == outline:
                        print(f"    in {link['source']}", file=sys.stderr)
                        break
            print()
        return False

    elif conflicts:
        print("\n❌ CONFLICTS FOUND:", file=sys.stderr)
        print(f"Found {len(conflicts)} word(s) with conflicting outlines:\n", file=sys.stderr)

        for word in sorted(conflicts.keys()):
            conflict_info = conflicts[word]
            print(f"  Word: '{word}'", file=sys.stderr)
            for outline in conflict_info['outlines']:
                # Find sources for this word-outline pair
                for link in all_links:
                    if link['word'] == word and link['outline'] == outline:
                        print(f"    - '{outline}' in {link['source']}", file=sys.stderr)
                        break
            print()

        return False
    else:
        print("\n⚠️ MISSING OUTLINES:", file=sys.stderr)
        print(f"Found {len(missings)} word(s) with missing outlines:\n", file=sys.stderr)

        for word in sorted(missings.keys()):
            missing_info = missings[word]
            print(f"  Word: '{word}'", file=sys.stderr)

    # No conflicts - create theory.json
    print("\n✓ No conflicts found!", file=sys.stderr)
    print(f"Generating theory.json with {len(all_links)} entries...", file=sys.stderr)

    # Create outline -> word mapping
    outline_to_word = {}
    for link in all_links:
        outline = link['outline']
        word = link['word']

        # If outline already exists with different word, keep the first
        if outline not in outline_to_word:
            outline_to_word[outline] = word

    # Write to build/theory.json
    output_path = build_dir / 'theory.json'
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(outline_to_word, f, indent=2, sort_keys=True)

    print(f"✓ Created {output_path}", file=sys.stderr)
    return True


if __name__ == '__main__':
    all_links, missings, invalids, conflicts = validate_theory(THEORY_DIR)
    success = output_results(all_links, missings, invalids, conflicts, BUILD_DIR, THEORY_DIR)

    sys.exit(0 if success else 1)
