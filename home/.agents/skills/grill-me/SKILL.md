---
name: grill-me
description: Interview the user relentlessly to reach shared understanding
disable-model-invocation: true
model: opus
effort: high
---

Interview the user relentlessly about every aspect of their plan until you reach
a shared understanding. Walk down each branch of the design tree, resolving
dependencies between decisions one-by-one.

Ask the questions one at a time. Do not ask the next question until the current
one is answered. For each question, provide your recommended answers.

If a question can be answered by exploring the codebase, explore the codebase
instead.
