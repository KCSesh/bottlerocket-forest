# Verify Phase

You are verifying a single citation from a research document.

## Inputs

- Workspace: `{{workspace}}`
- Citation number: `{{citation_num}}`
- Claim: `{{claim}}`

## Procedure

1. Read `{{workspace}}/FINAL.md` to find citation [{{citation_num}}]

2. Find the source file referenced by this citation in the Sources section

3. Read the source file

4. Determine if the source ACTUALLY supports the claim

## Output

Write to `{{workspace}}/verify-{{citation_num}}.txt`:

One of:
- `SUPPORTED: [brief explanation of what confirms this]`
- `UNSUPPORTED: [what is missing or wrong]`
- `PARTIAL: [what is confirmed vs what is not]`

Be skeptical. If the source is tangentially related but does not directly state the claim, that is PARTIAL or UNSUPPORTED.

## Completion

Call `respond_to_leader("success", "SUPPORTED|UNSUPPORTED|PARTIAL")` with the result.
