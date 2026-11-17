// A small checker for the shape of what the Rust commands send (row A-09). A
// value from `JSON.parse` is `unknown`; `parse` checks it against a description of
// the TypeScript type and gives it back as that type.
//
// The description of an object has to list every field of the type (the compiler
// says when one is missing), and each field is checked for its primitive type, so
// a field that Rust renames, drops or changes shows up as a named problem, for
// example `view.report.summary.pdf: expected a string or null, got undefined`.
// Fields that are in the JSON and not in the type are not a problem: the page
// does not read them.

type Problems = string[];

export interface Check<T> {
  (value: unknown, at: string, problems: Problems): void;
  /** For the compiler only. */
  readonly type?: T;
}

export type Fields<T> = { [K in keyof T]-?: Check<T[K]> };

function describe(value: unknown): string {
  if (value === null) {
    return "null";
  }
  return Array.isArray(value) ? "an array" : typeof value;
}

function primitive<T>(name: string, test: (value: unknown) => boolean): Check<T> {
  return (value, at, problems) => {
    if (!test(value)) {
      problems.push(`${at}: expected ${name}, got ${describe(value)}`);
    }
  };
}

export const string = primitive<string>("a string", (v) => typeof v === "string");
export const number = primitive<number>("a number", (v) => typeof v === "number" && Number.isFinite(v));
export const boolean = primitive<boolean>("a boolean", (v) => typeof v === "boolean");

export function nullable<T>(check: Check<T>): Check<T | null> {
  return (value, at, problems) => {
    if (value !== null) {
      const found: Problems = [];
      check(value, at, found);
      // Say what was expected in full, so that `undefined` is not read as "missing is fine".
      problems.push(...found.map((p) => p.replace(": expected ", ": expected null or ")));
    }
  };
}

export function array<T>(check: Check<T>): Check<T[]> {
  return (value, at, problems) => {
    if (!Array.isArray(value)) {
      problems.push(`${at}: expected an array, got ${describe(value)}`);
      return;
    }
    value.forEach((item, i) => check(item, `${at}[${i}]`, problems));
  };
}

export function literal<T extends string | number>(...allowed: T[]): Check<T> {
  return (value, at, problems) => {
    if (!allowed.includes(value as T)) {
      problems.push(`${at}: expected one of ${allowed.map((a) => JSON.stringify(a)).join(", ")}, got ${JSON.stringify(value)}`);
    }
  };
}

/** An object with every field of `T`. */
export function object<T>(fields: Fields<T>): Check<T> {
  return (value, at, problems) => {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      problems.push(`${at}: expected an object, got ${describe(value)}`);
      return;
    }
    const record = value as Record<string, unknown>;
    for (const [key, check] of Object.entries(fields) as [string, Check<unknown>][]) {
      if (!(key in record)) {
        problems.push(`${at}.${key}: the field is missing`);
      } else {
        check(record[key], `${at}.${key}`, problems);
      }
    }
  };
}

/** An object that has a field with the name of its kind, like `{ kind: "text"; text: string }`. */
export function tagged<T extends Record<Tag, string>, Tag extends keyof T & string>(
  tag: Tag,
  cases: { [V in T[Tag]]: Fields<Omit<Extract<T, Record<Tag, V>>, Tag>> },
): Check<T> {
  return (value, at, problems) => {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      problems.push(`${at}: expected an object, got ${describe(value)}`);
      return;
    }
    const kind = (value as Record<string, unknown>)[tag];
    const fields = typeof kind === "string" ? (cases as Record<string, Fields<unknown>>)[kind] : undefined;
    if (fields === undefined) {
      problems.push(`${at}.${tag}: expected one of ${Object.keys(cases).join(", ")}, got ${JSON.stringify(kind)}`);
      return;
    }
    object<Record<string, unknown>>(fields as Fields<Record<string, unknown>>)(value, at, problems);
  };
}

/** The problems of a value, or an empty list. */
export function problemsOf<T>(check: Check<T>, value: unknown, at: string): string[] {
  const problems: Problems = [];
  check(value, at, problems);
  return problems;
}

/** The value as `T`, or an error that lists every problem. */
export function parse<T>(check: Check<T>, value: unknown, at: string): T {
  const problems = problemsOf(check, value, at);
  if (problems.length > 0) {
    throw new Error(`${at} is not the shape the page expects:\n${problems.join("\n")}`);
  }
  return value as T;
}
