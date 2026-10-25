export type ClassValue = string | false | null | undefined;

/** Joins class names, dropping falsy entries. */
export const cx = (...values: ClassValue[]): string => values.filter(Boolean).join(" ");
