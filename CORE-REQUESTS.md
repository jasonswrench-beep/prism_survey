# Requests to the shared core

What Prism Survey needs from `openstage-core` that the core does not offer yet. Nothing here is built in Prism Survey in its
place: each is waiting on the core. Read the core's `CORE-NOTES.md` for what it offers now. Newest first.

## 1. C8, the local AI runtime (for phase 8, the optional model)

**Asked 17 November 2026. Core state: on the roadmap, agreed, not scheduled.**

The specification turns the model off by default and, when it is turned on, uses the suite's local AI runtime first and a key
the researcher supplies second. Prism Survey does not keep keys of its own and does not ship a runtime, so phase 8 waits on C8.
Everything works without it today: detection, the self-check and the recommendations are rules, and the researcher confirms
every field.

What Prism Survey needs to know from C8 before it builds phase 8:

- **How a tool asks whether a local model or a key is there, and calls it,** with no model meaning every feature still works.
- **Whether the call can carry a page image as well as text.** Scanned articles are read by OCR now; a model that can look at the
  page would help most where the OCR text is poor.
- **Structured answers.** Each of Prism Survey's uses asks for a fixed shape (a ranked list, the fields of a scale, a direction
  per item), so a way to ask for JSON that matches a schema, or a stated way to check and retry, matters more than free text.
- **Where the key settings page lives,** so a researcher sets a key once for the suite.
- **How a send to a key-based service is approved.** Journal articles are copyrighted, so the specification asks that, before
  any page text goes to a key-based service, the tool says what will be sent and asks, once per document. If C8 has its own
  approval step, Prism Survey would use it and keep its once-per-document rule on top; if not, Prism Survey asks itself.

What Prism Survey would add itself, once C8 says how a model is called:

1. **Ranking candidates** on a page of an article, beside the rule-based score, shown as a suggestion.
2. **Parsing a region the rules cannot** (a garbled table, a scanned page) into the scale's fields, which then go through the
   confirm screen like any other import.
3. **Judging item direction** in the self-check: a model's view on which items are reverse-worded, shown as a finding with its
   reason, never applied without the researcher.
4. **A fourth basis for recommendations:** a model's reading of what would sit well beside the study's scales, listed with the
   rule-based suggestions and marked as the model's.
5. **Worked examples from the researcher's own corrections,** which stay on the machine and are sent to a local model only.
6. **The benchmark with and without the model,** so the rules alone are always measured too.

The rules Prism Survey keeps whatever C8 says: the model is off until the researcher turns it on; nothing from a model enters the
library without the confirm screen; a model's suggestion is a finding the researcher accepts or dismisses; and nothing leaves
the machine without the researcher's say, asked once per document for page text.
