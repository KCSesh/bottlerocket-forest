#!/usr/bin/env python3
"""State machine for deep-research skill."""
import json
import re
import sys
from pathlib import Path

def parse_scout_subquestions(scout_path):
    """Extract sub-questions from 00-scout.md."""
    if not scout_path.exists():
        return []
    content = scout_path.read_text()
    questions = []
    in_subq = False
    current = {}
    for line in content.split('\n'):
        if line.startswith('### ') and re.match(r'### \d+\.', line):
            if current:
                questions.append(current)
            current = {'text': re.sub(r'^### \d+\.\s*', '', line)}
        elif current and line.startswith('- **Type:**'):
            current['type'] = line.split(':', 1)[1].strip().lower()
        elif current and line.startswith('- **Key files:**'):
            current['files'] = line.split(':', 1)[1].strip()
    if current:
        questions.append(current)
    return questions

def count_research_files(workspace):
    """Count NN-*.md files excluding 00-scout.md."""
    return len([f for f in workspace.glob('[0-9][0-9]-*.md') if f.name != '00-scout.md'])

def extract_citations(final_path):
    """Extract citation numbers from FINAL.md."""
    if not final_path.exists():
        return []
    content = final_path.read_text()
    return sorted(set(re.findall(r'<sup>\[(\d+)\]</sup>', content)), key=int)

def main():
    workspace = Path(sys.argv[1])
    progress_file = workspace / 'progress.json'
    
    if progress_file.exists():
        state = json.loads(progress_file.read_text())
    else:
        state = {'phase': 'scout', 'researched': [], 'verified': []}
    
    def save_and_continue():
        progress_file.write_text(json.dumps(state, indent=2))
    
    phase = state['phase']
    
    # SCOUT PHASE
    if phase == 'scout':
        scout_file = workspace / '00-scout.md'
        if not scout_file.exists():
            print(json.dumps({
                'type': 'spawn',
                'prompt': 'Execute the scout phase for this research task.',
                'context_files': ['skills/deep-research/phases/SCOUT.md'],
                'context_data': {'workspace': str(workspace)},
                'output_file': '00-scout.md'
            }))
            return
        # Gate: scout must have sub-questions
        subqs = parse_scout_subquestions(scout_file)
        if not subqs:
            print(json.dumps({'type': 'gate_failed', 'reason': 'Scout produced no sub-questions'}))
            return
        state['subquestions'] = subqs
        state['phase'] = 'research'
        save_and_continue()
    
    # RESEARCH PHASE
    if state['phase'] == 'research':
        subqs = state.get('subquestions', [])
        researched = state.get('researched', [])
        for i, sq in enumerate(subqs):
            if i in researched:
                continue
            file_num = str(i + 1).zfill(2)
            slug = re.sub(r'[^a-z0-9]+', '-', sq['text'].lower()[:40]).strip('-')
            output_file = f'{file_num}-{slug}.md'
            print(json.dumps({
                'type': 'spawn',
                'prompt': f'Research this sub-question: {sq["text"]}',
                'context_files': ['skills/deep-research/phases/RESEARCH.md', f'{workspace}/00-scout.md'],
                'context_data': {
                    'workspace': str(workspace),
                    'subquestion': sq['text'],
                    'subquestion_type': sq.get('type', 'fact-find'),
                    'key_files': sq.get('files', ''),
                    'output_file': output_file
                },
                'output_file': output_file
            }))
            state['researched'].append(i)
            save_and_continue()
            return
        # Gate: all sub-questions answered
        expected = len(subqs)
        actual = count_research_files(workspace)
        if actual < expected:
            print(json.dumps({'type': 'gate_failed', 'reason': f'Only {actual}/{expected} sub-questions answered'}))
            return
        state['phase'] = 'assemble'
        save_and_continue()
    
    # ASSEMBLE PHASE
    if state['phase'] == 'assemble':
        final_file = workspace / 'FINAL.md'
        if not final_file.exists():
            research_files = sorted([str(f) for f in workspace.glob('[0-9][0-9]-*.md')])
            print(json.dumps({
                'type': 'spawn',
                'prompt': 'Assemble the research into a final document.',
                'context_files': ['skills/deep-research/phases/ASSEMBLE.md'] + research_files,
                'context_data': {'workspace': str(workspace)},
                'output_file': 'FINAL.md'
            }))
            return
        state['phase'] = 'verify'
        save_and_continue()
    
    # VERIFY PHASE
    if state['phase'] == 'verify':
        final_file = workspace / 'FINAL.md'
        citations = extract_citations(final_file)
        verified = state.get('verified', [])
        final_content = final_file.read_text()
        for cit in citations:
            if cit in verified:
                continue
            # Find the claim for this citation
            pattern = rf'([^.]*<sup>\[{cit}\]</sup>[^.]*\.)'
            match = re.search(pattern, final_content)
            claim = match.group(1) if match else f'Citation [{cit}]'
            print(json.dumps({
                'type': 'spawn',
                'prompt': f'Verify citation [{cit}]',
                'context_files': ['skills/deep-research/phases/VERIFY.md', f'{workspace}/FINAL.md'],
                'context_data': {
                    'workspace': str(workspace),
                    'citation_num': cit,
                    'claim': claim
                },
                'output_file': f'verify-{cit}.txt'
            }))
            state['verified'].append(cit)
            save_and_continue()
            return
        state['phase'] = 'done'
        save_and_continue()
    
    # DONE
    if state['phase'] == 'done':
        print(json.dumps({'type': 'done'}))
        return

if __name__ == '__main__':
    main()
