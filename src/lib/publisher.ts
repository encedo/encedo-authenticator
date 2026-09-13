// Who publishes this app, for the About screen and for the two store listings.
// Kept in one place because the same four strings are asked for by Google Play,
// App Store Connect and the app itself.
//
// The company is moving from Encedo to RKV (docs/PLAN-RKV.md, decisions A1–A7),
// so the fields that need a legal answer are marked and the screen says out loud
// that they are not settled. Nothing here is invented: what is unknown is empty.

export interface Publisher {
  /** The name on the store listing and in the app. */
  name: string;
  /** Legal entity, as it goes on the listing. Empty until RKV is decided (A3). */
  legal: string;
  /** Where a person writes about this app. Empty until the RKV mailbox exists. */
  contact: string;
  /** Privacy policy, which both stores require before publishing (A7). */
  privacy: string;
  /** Terms, where they exist. */
  terms: string;
}

export const publisher: Publisher = {
  name: "Encedo",
  legal: "",
  contact: "",
  privacy: "",
  terms: "",
};

/** What still has to be filled in before a store will take the app. */
export function missingForStore(p: Publisher = publisher): string[] {
  const missing: string[] = [];
  if (!p.legal) missing.push("legal name");
  if (!p.contact) missing.push("contact address");
  if (!p.privacy) missing.push("privacy policy");
  return missing;
}
