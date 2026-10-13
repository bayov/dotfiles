---
name: phrasing
description: A focused rewrite pass that makes text short, simple and plain, for when a first draft still came out too long or too hard to follow. Use when the user types /phrasing, asks to explain or summarize "in simple terms", "concisely", "more simply", says "I don't understand", "too verbose", "too long", "gets clipped", or asks for a pass over the UI text, tooltips or report prose of a module.
---

# Phrasing

The rules for simple, concise writing are in the global AGENTS.md ("Simple and concise
from the first draft"), and every draft should already follow them. When the user still
asks for this, the first version failed them. So this pass goes further than the first
draft did.

The user's words when they ask:

- "in simple terms what did you rule out and what is still possible?"
- "Can you explain more simply. I dont understand your questions and the bottom line"
- "Summarize each bug and fix. VERY SIMPLY AND CONCISELY"
- "This text is too verbose, just write something simple and concise"
- "AI tends to write long complicated hard to understand text, and our goal is organic,
  simple and concise text, that gets the point across quick for humans reading it"

## 1. Pick what to rephrase

1. **No argument, or "that" / "this".** Your own last answer.
2. **Quoted or pasted text, or a screenshot of UI.** That text. When it lives in code,
   find the string and edit it.
3. **A module, file, page or report.** A copy pass over every string a reader sees there
   (labels, tooltips, descriptions, status lines, chat and alert lines, empty states,
   report notes). Edit them in place and run the tests when strings are asserted.

## 2. Rewrite from the meaning

Don't edit the old sentences word by word. Trimming a few words is what keeps text long.
Instead:

1. Write down, for yourself, the one thing the reader must take away, and what they do
   with it.
2. Keep every fact that changes what they do or decide. Drop the rest: how it works
   inside, history, hedges, caveats that rarely matter, and anything already on screen.
3. Write it fresh, the way you'd say it out loud to a friend. Short common words. One
   idea per sentence.
4. Expect the result to be a lot shorter, often half or less. When it isn't, look again
   at what you kept.

When the user said "explain X" or "I don't understand", shorter is not enough. Say it a
different way, with a concrete example from their case.

## 3. Check it as the reader

Read the result as the person it's for: the user who didn't follow the first answer, or a
player glancing at a tooltip mid-fight.

- Is the bottom line in the first sentence?
- Any word they might not know? Code names, internal terms (pin, pick, board, job, slot),
  abbreviations they never used? Replace them.
- Any sentence with two ideas? Split it.
- Any LLM mannerism? Fragment-colon punch lines, packed clauses, clever verbs ("caps",
  "owns", "absorbs"). Rewrite them plainly. Concise is not compressed.
- UI text: does it fit the space it has? Count the width when unsure.

## More for UI text

These add to the rules in AGENTS.md.

- Rules and requirements in a tooltip are a few words each: "Needs Shaman level 32+.",
  "One curse per warlock.", "Doesn't stack with Sunder Armor, which wins."
- Counts go in parentheses: "BUILT IN (1)".
- No prefix words that read awkwardly: "now Shaman leveling" is "Shaman leveling".
- Use one name per thing across the whole module.
- Formatting helps the eye: color for state (orange near a limit, red at it), names in
  their own color, the number that matters bright.
- Examples the user wrote:
  - `Healthstone ({zone}) saves 2m 33s`
  - `Time until below 15% mana`
  - `Click to update Auctionator "Supplies"`
  - `Entered new Ragefire Chasm instance (3 out of 10 this hour)`
  - `Back in same Ragefire Chasm instance (started 12m ago, 3 out of 10 this hour)`
  - `Leave it`

## 4. Report

- For an answer: the rewritten answer and nothing else. No "Here's a simpler version".
- For UI text and copy passes: a before / after table of what changed, grouped by file or
  page. Then list anything you left long on purpose, and why.
