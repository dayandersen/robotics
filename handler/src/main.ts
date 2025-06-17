import { routeRequest } from "./request_router.ts";

let requestCounter = 0;
// Learn more at https://docs.deno.com/runtime/manual/examples/module_metadata#concepts
if (import.meta.main) {
  Deno.serve({ port: 8000 }, async (req) => {
    requestCounter++;
    return await routeRequest(req, requestCounter);
  });
}
