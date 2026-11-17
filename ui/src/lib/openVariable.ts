import { createContext } from "react";

/**
 * Opens the environment view on the row of the variable named, or null on a
 * page whose tool has no environment view. A failure card reads it to link
 * each declared variable that its sentences name.
 */
export const OpenVariable = createContext<((name: string) => void) | null>(null);
