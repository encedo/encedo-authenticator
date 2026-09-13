// Who publishes this app, for the About screen and for the two store listings.
// Kept in one place because the same handful of strings is asked for by Google
// Play, App Store Connect and the app itself.
//
// Taken from rkv.pl on 13 September 2026. What the site does not state is left
// empty rather than guessed, and the About screen says which of those a store
// will still ask for.

/** The name of the app itself, which is not the name of the company (PLAN-RKV, A3). */
export const product = "Encedo HEM Authenticator";

export interface Publisher {
  /** The short name, as it reads in the app. */
  name: string;
  /** Legal entity, as it goes on a listing. */
  legal: string;
  /** Registered address. */
  address: string;
  /** Register entries a Polish company is identified by. */
  registration: string;
  /** Where a person writes about this app. */
  contact: string;
  /** Privacy policy, which both stores require before publishing (PLAN-RKV, A7). */
  privacy: string;
  /** Terms, where they exist. */
  terms: string;
}

export const publisher: Publisher = {
  name: "RKV",
  legal: "RKV spółka z ograniczoną odpowiedzialnością",
  address: "ul. gen. Stefana Grota-Roweckiego 10/12, 52-220 Wrocław, Poland",
  registration: "KRS 0001173474 · NIP 8993025480 · REGON 541756532",
  contact: "office@rkv.pl",
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
