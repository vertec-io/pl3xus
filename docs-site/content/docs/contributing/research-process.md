---
title: Research Process
---
# Research Process

This document describes how to conduct and document research for pl3xus development.

---

## Overview

The research process ensures:
- Design decisions are documented and reviewable
- Implementation is informed by thorough analysis
- Knowledge is preserved for future reference
- Work can be archived with clear conclusions

---

## Directory Structure

```
research/
├── README.md              # Current status, entry points
├── NEXT_SESSION_TASKS.md  # Tasks for next development session
│
├── active/                # Currently active research
│   └── [topic-name]/      # One folder per research topic
│       ├── README.md      # Overview and current status
│       ├── 1-problem.md   # Problem definition
│       ├── 2-options.md   # Options considered
│       └── ...            # Numbered documents
│
├── status/                # Dated status snapshots
│   └── YYYY-MM-DD-description.md
│
└── archive/               # Completed research
    ├── _CONCLUSION_TEMPLATE.md  # Template for conclusions
    └── YYYY-MM-[topic]/         # Archived topic folders
        ├── CONCLUSION.md        # Summary (use template)
        └── *.md                 # Original research docs
```

---

## Starting New Research

### 1. Create Active Research Folder

```bash
mkdir -p research/active/[topic-name]
```

### 2. Create README.md

```markdown
# [Topic Name] Research

**Started**: YYYY-MM-DD  
**Status**: 🔬 In Progress | 📝 Draft | ✅ Ready for Review

## Problem Statement

[Brief description of what we're trying to solve]

## Current Progress

- [ ] Problem definition
- [ ] Options analysis
- [ ] Proposed solution
- [ ] Implementation plan

## Documents

| # | Document | Description |
|---|----------|-------------|
| 1 | [problem.md](./1-problem.md) | Problem definition |
| 2 | [options.md](./2-options.md) | Options considered |

## Quick Decision

[Once decided, summarize the key decision here]
```

### 3. Number Your Documents

Use numbered prefixes for reading order:
- `1-problem-statement.md`
- `2-current-state-analysis.md`
- `3-options-considered.md`
- `4-proposed-solution.md`
- `5-implementation-plan.md`

---

## Research Document Guidelines

### Problem Statement

Answer:
- What exactly is the problem?
- Who is affected?
- What's the impact if not solved?
- What's the scope?

### Options Analysis

For each option:
- Description
- Pros and cons
- Effort estimate
- Risk assessment

### Proposed Solution

Include:
- Recommended option and why
- High-level design
- Key code changes
- Migration path (if applicable)

### Implementation Plan

Include:
- Step-by-step tasks
- Estimated effort
- Dependencies
- Success criteria

---

## Status Updates

Create status snapshots at significant milestones:

```bash
touch research/status/YYYY-MM-DD-description.md
```

Format:
```markdown
# Status: [Description]

**Date**: YYYY-MM-DD

## Completed
- [x] Task 1
- [x] Task 2

## In Progress
- [ ] Task 3

## Blockers
- [Issue description]

## Next Steps
1. Step 1
2. Step 2
```

---

## Archiving Research

When research is complete (implemented, rejected, or deferred):

### 1. Create Archive Folder

```bash
mkdir research/archive/YYYY-MM-[topic-name]
```

### 2. Move All Documents

```bash
mv research/active/[topic-name]/* research/archive/YYYY-MM-[topic-name]/
rmdir research/active/[topic-name]
```

### 3. Create CONCLUSION.md

Use `research/archive/_CONCLUSION_TEMPLATE.md` as starting point.

**Key sections:**
- Problem Statement
- Options Considered
- Final Implementation
- Critical Decisions
- Outstanding/Deferred Items
- Important Nuance

### 4. Update Archive README

Add entry to `research/archive/README.md`.

---

## Best Practices

### Do

✅ Start with clear problem statement  
✅ Document all options considered  
✅ Include code examples  
✅ Number documents for reading order  
✅ Create CONCLUSION.md when archiving  
✅ Update status/ regularly  

### Don't

❌ Delete research documents (archive instead)  
❌ Skip the conclusion when archiving  
❌ Leave research in active/ indefinitely  
❌ Mix multiple unrelated topics in one folder  

---

## Templates

### Active Research README

See: `research/active/meteorite-analysis/README.md` for example

### Conclusion Document

See: `research/archive/_CONCLUSION_TEMPLATE.md`

### Status Update

See: `research/status/` for examples

---

**Last Updated**: 2025-12-07

