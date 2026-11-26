import { z } from "zod";

// zod compiles fast parsers with `new Function` when the page allows it, and
// its probe for that permission breaks a Content-Security-Policy that forbids
// eval (EMB-6). The probe runs when a schema is built, so every module here
// that builds a schema, the generated ones included, imports zod from this
// module, and the setting applies before any schema is built.
z.config({ jitless: true });

export { z };
