---
name: align
description: Build a shared understanding and a ubiquitous language.
disable-model-invocation: true
---

Interview Session
=================

Interview the user relentlessly about every aspect of their plan until you reach
a shared understanding. Walk down each branch of the design tree, resolving
dependencies between decisions one-by-one. For each question, provide your
recommended answer.

Ask the questions one at a time, waiting for feedback on each question before
continuing.

If a question can be answered by exploring the codebase, explore the codebase
instead.

Ubiquitous Language
-------------------

Look for relevant `GLOSSARY.md` files which describe the project's ubiquitous
language. As part of aligning with the user, you need to build a shared
understanding of the domain and a precise vocabulary for discussing it.

During the session
------------------

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific
scenarios. Invent scenarios that probe edge cases and force the user to be
precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you
find a contradiction, surface it: "Your code cancels entire Orders, but you just
said partial cancellation is possible — which is right?"

### Challenge against the glossary

When the user uses a term that conflicts with the existing language in
`GLOSSARY.md`, call it out immediately. "GLOSSARY.md defines 'cancellation' as
X, but you seem to mean Y — which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term.
"You're saying 'account' — do you mean the Customer or the User? Those are
different things."

### Update GLOSSARY.md inline

When a term is resolved, update `GLOSSARY.md` right there. Don't batch these up
— capture them as they happen. Follow the format in
[GLOSSARY_FORMAT.md](./GLOSSARY_FORMAT.md).

Don't couple `GLOSSARY.md` to implementation details. Only include terms that
are meaningful to domain experts.

Interview session persistence
-----------------------------

Store the state of the interview session (questions asked, answers given,
glossary updates) in `ALIGN.md`. This allows the session to be paused and
resumed, and provides a record of the shared understanding that was built so
far.

Update `ALIGN.md` after each question-answer or glossary update. Use format:

```md
Align Session
=============

Brief summary of plan being discussed.

Interview session
-----------------

### Q1: Title of question

Additional question details (if needed).

**Answer:**

Answer details, including any follow-up questions that arose.
```
