import { z } from "zod";

// zod compiles fast parsers with `new Function` when the page allows it, and
// its probe for that permission breaks the server's Content-Security-Policy
// (EMB-6). Every module that builds a schema imports zod from here, so the
// setting applies before any schema is built.
z.config({ jitless: true });

export { z };
