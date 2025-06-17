export async function handleFavIcon(): Promise<Response> {
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
