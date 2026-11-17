# Self-check fixtures

Synthetic stored scales, written in the layout of the stored scale files, that recreate each worked example of the specification's scoring self-check in new words. None is taken from a real measure. `crates/prismsurvey-import/tests/self_check_fixtures.rs` imports each one and checks that the self-check reports the finding named here, and nothing on the two guards.

| File | Check | What the self-check should report |
|---|---|---|
| 01-recode-against-wording | Recode list against wording | Item 4 is recoded but reads with the construct. |
| 02-recode-swapped-odd-even | Recode list against wording | Part of the list is swapped: items 9 and 10 are worded positively but recoded, and 11 and 12 are negated but left out. |
| 03-recode-missing | Recode list missing | Items 1, 4 and 6 are worded against shouting and there is no recode line. |
| 04-late-added-item | Late-added item | Item 7, the last, reads like the recoded items ("allowed") but is not on the list. |
| 05-repeats-in-a-list | Repeats in a list | 7 is listed twice; item 8 is worded negatively and missing, so the second 7 was likely meant as 8. |
| 06-duplicate-items | Duplicate items | Item 5 repeats item 1 and item 6 repeats item 2, word for word. |
| 07-one-subscale-per-item-pass | One subscale per item | Passes: six items split three and three with none left out. |
| 08-subscale-names-agree | Subscale names agree | The second factor is "Warm" in the scoring lines and "Friendly" under the factor table. |
| 09-Garden Worry-5 | Item count against title | The file name says 5, the title says 6, and there are 6 items. |
| 10-score-range-pass | Score range | Passes: 5 to 25 for 5 items on 5 points. |
| 11-score-range-missing | Score range | No range stated; the tool supplies 8 to 40. |
| 12-instructions-against-format | Instructions against format | Box grid instructions mention "blanks". |
| 13-leftover-source | Leftover source | A blank template still carries a source. |
| 14-typing-slips | Typing slips | Item 2 begins ". ", item 3 reads "did not used", the total name reads "confidance". |
| 15-score-names-agree | Score names agree | The total is both the "composite Theatre Nerves score" and the "Overall Stage Fright Score". |
| 16-guard-negation-not-reversed | Guard | Item 4 ("I don't respect...") has a negation but scores with the construct; it is correctly not recoded. No finding. |
| 17-guard-reversed-without-negation | Guard | Item 4 ("...is limited") has no negation word but is correctly recoded. No finding. |
