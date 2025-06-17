let requestCounter = 0;
// Learn more at https://docs.deno.com/runtime/manual/examples/module_metadata#concepts
if (import.meta.main) {
  Deno.serve({port: 8000}, async (req) => {
    const url = new URL(req.url);

    switch (url.pathname) {
      case '/favicon.ico':
        return handleFavIcon()
        
      default:
        requestCounter++;
        console.log(`Our ${requestCounter}th was ${JSON.stringify(req)}`) 

        switch (url.pathname) {
          case '/turn-on-led':

          case '/turn-off-led':

          default: 
            return new Response(`Hello to our ${requestCounter}th user!`);
        }
    }
  });
}

async function handleFavIcon() {
  try {
        // Serve favicon from file system
        const faviconFile = await Deno.readFile('./favicon.ico');
        return new Response(faviconFile, {
          headers: {
            'Content-Type': 'image/x-icon',
            'Cache-Control': 'public, max-age=86400' // Cache for 1 day
          }
        });
      } catch {
        // If favicon file doesn't exist, return 404
        return new Response('Not Found', { status: 404 });
      }
}
