import { handleFavIcon } from "./favicon_handler.ts";
import { turnOnLed, turnOffLed, getLedStatus } from "./esp32_client.ts";

export async function routeRequest(req: Request, requestCounter: number): Promise<Response> {
  const url = new URL(req.url);

  switch (url.pathname) {
    case "/favicon.ico":
      return handleFavIcon();
    case "/turn-on-led":
      await turnOnLed();
      return new Response("LED turned on", { status: 200 });
    case "/turn-off-led":
      await turnOffLed();
      return new Response("LED turned off", { status: 200 });
    case "/led-status":
      const status = await getLedStatus();
      return new Response(status, { status: 200 });
    case "/":
      return new Response(`Hello to our ${requestCounter}th user!`);
    default:
      console.log(`Our ${requestCounter}th request was ${JSON.stringify(req)}`);
      return new Response("Not Found", { status: 404 });
  }
}
