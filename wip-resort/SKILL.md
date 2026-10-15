---
name: wip-resort
description: Re-sort the MASTER_WIP board so jobs sit in the right production section, under the right installer banner, with Hold jobs at the bottom of each group.
---

# WIP Re-sort

Re-sorts the `MASTER_WIP` sheet in `WIP_MASTER.xlsx` so every job sits in the correct section and under the correct installer banner.

## When to use

When the user asks to tidy, re-sort, or re-file the WIP board — e.g. "jobs are in the wrong section", "sort the WIP", "re-file these by installer", "run the WIP re-sort". Also triggered by `/wip-resort`.

## Sheet structure (do not break this)

`MASTER_WIP` is a visual board, not a flat table. Row types:

| Type | How to detect | Styling |
|---|---|---|
| Header | Row 1 | fill `#FFC000`, black bold |
| Section banner | Only col A filled, and the value is in the section list | fill `#FFD966`, black bold 12pt |
| Installer-group banner | Only col B filled | fill `#FFF2CC`, black bold 11pt |
| Blank separator | All cells empty — exactly one above each section banner (not the first) | — |
| Job row | Anything else | status + installer coloured by conditional formatting |

Columns (A:S): Section, Installer Group, JOB BAG, CUSTOMER, BRANDING / JOB INFO, Production Status, VIN_STOCK_VPO, PO, INSTALLER, Install Location & Contact, Dealership, Con Note, Notes, Raised By, Scheduled Date, Completed Date, Date Scheduled, Invoice No, Source Row.

Sections in board order:
`ART / PROOFING`, `WAITING ON INFO FOR ART / PRODUCTION`, `ON APPROVAL`, `IN PRODUCTION`, `SUPPLY ONLY`, `SENT - REQUIRES SCHEDULING`, `ON HOLD`.

## Rules

Compare statuses case-insensitively. Normalise names with
`canon(s) = lowercase, replace [_\-()] with space, collapse spaces, trim`.

**1. Left production → move to `SENT - REQUIRES SCHEDULING`**
Any job in `IN PRODUCTION` whose status is:
`Ready, Ready On Rack, Ready/Installing, Sent, Installed, Complete, Completed, Done, Closed, Closed (No Charge), No Charge, Invoiced, Already Invoiced, JC INV, JC INV via Xero, Invoiced in June`

**2. Not left production → move back to `IN PRODUCTION`**
Any job in `SENT - REQUIRES SCHEDULING` whose status is:
`In Production, Finishing, Reprint`

**3. Installer banners — exact matches only**
Move a job to the banner whose `canon(name)` equals `canon(INSTALLER)`, when such a banner exists in that section. This deliberately catches punctuation variants (`Signature_Wrapz` = `Signature Wrapz`, `Alan De-signs` = `Alan De-Signs`).

Do **not** guess. Banners are a mix of individuals (`Nathan`, `Craig Denning`, `Joel`, `Col Booth`) and teams/regions (`Vic Team`, `Vic Regional`, `QLD Regional`, `ACT`, `Tasmania`, `Elite`) that legitimately hold many different installers. If a job's installer has no exact banner in that section, leave it where it is and **add it to the review list** — report those to the user at the end and ask, never auto-move them.

When a job moves between sections, place it under: the banner matching its installer if one exists in the destination; otherwise a banner with the same name as its source group; otherwise create that banner.

**4. Hold to the bottom**
Within each installer group, jobs with status `Hold` sort last. Preserve the existing relative order of everything else (stable sort).

**5. Drop empty banners**
A section or installer banner with no job rows under it is removed.

Done when: a verification pass reports 0 post-production statuses left in `IN PRODUCTION`, 0 production statuses left in `SENT - REQUIRES SCHEDULING`, and 0 groups where a non-Hold job appears after a Hold job.

## Steps

1. **Read** `MASTER_WIP` A1 to the end of the used range via `execute_office_js`. Parse into a model of sections → groups → job rows, classifying each row by the table above.

   Done when: section count, group count and job count are known and the job count matches the non-banner non-blank rows.

2. **Apply rules 1–4** in memory. Record every move as `{row, status, installer, from, to}`, and build the review list from rule 3.

   Done when: you can report counts for moved-to-SENT, moved-to-IN PRODUCTION, and regrouped.

3. **Report the dry run** to the user before writing — the three counts plus the review list. If any review item looks like a real mis-file, ask before proceeding.

   Done when: the user has seen the counts.

4. **Rebuild and write.** Emit header, then for each section: blank separator (except before the first), section banner, any ungrouped jobs, then each group's banner followed by its jobs. Clear the sheet with `range.clear(Excel.ClearApplyTo.all)` first, then write in chunks of 250 rows.

   Done when: the written row count equals the assembled row count.

5. **Re-apply formatting** — it is destroyed by the clear:
   - Header `#FFC000`, section banners `#FFD966`, group banners `#FFF2CC`, all black bold
   - Status conditional formats on col F and installer conditional formats on col I, sourced from the `LISTS` sheet (statuses `LISTS!$A$2:$A$28`, installers `LISTS!$B$2:$B$84`; installer colours are the fills on `LISTS` col B)
   - Dropdowns on col F and col I pointing at the same ranges, `ignoreBlanks: true`
   - JOB BAG (col C) as text (`numberFormat = "@"`)
   - Wrap text + top align, cap column widths at 200pt, `autofitRows()`, header row 32pt
   - Freeze row 1, apply `autoFilter` over the whole range (do NOT convert to a Table — banner rows break it)

   Done when: a read-back shows the header, one section banner and one group banner with the expected fills.

6. **Verify** by re-reading the sheet and checking the three conditions in "Done when" above. Report the section counts and the review list.

   Done when: all three counts are 0 and you have quoted the per-section job counts back to the user.

## Notes

- Take a backup copy of the sheet first if the user hasn't run this recently.
- Never sort via the AutoFilter dropdowns — that scrambles banner rows.
- `Source Row` is an audit column mapping back to the pre-clean layout; carry it through untouched.
