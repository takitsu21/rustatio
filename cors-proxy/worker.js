export default {
  async fetch(request, env, ctx) {
    // Handle CORS preflight requests
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'Access-Control-Allow-Origin': '*',
          'Access-Control-Allow-Methods': 'GET, HEAD, OPTIONS',
          'Access-Control-Allow-Headers': '*',
          'Access-Control-Max-Age': '86400',
        }
      });
    }

    // Only allow GET requests
    if (request.method !== 'GET') {
      return new Response('Method not allowed', { 
        status: 405,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }

    const url = new URL(request.url);
    const targetUrl = url.searchParams.get('url');
    
    if (!targetUrl) {
      return new Response('Missing url parameter. Usage: ?url=http://tracker.example.com/announce', { 
        status: 400,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }

    // Validate URL
    let targetURL;
    try {
      targetURL = new URL(targetUrl);
    } catch (e) {
      return new Response('Invalid URL', { 
        status: 400,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }

    // Only allow HTTP/HTTPS
    if (!['http:', 'https:'].includes(targetURL.protocol)) {
      return new Response('Only HTTP/HTTPS URLs are allowed', { 
        status: 400,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }

    try {
      // Fetch the target URL
      const response = await fetch(targetUrl, {
        method: 'GET',
        headers: {
          'User-Agent': 'RustatioWeb/1.0 (via Cloudflare Worker)'
        }
      });
      
      // Create new response with CORS headers
      const newResponse = new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
      });
      
      // Copy important headers
      const headersToKeep = ['content-type', 'content-length'];
      for (const header of headersToKeep) {
        const value = response.headers.get(header);
        if (value) {
          newResponse.headers.set(header, value);
        }
      }
      
      // Add CORS headers
      newResponse.headers.set('Access-Control-Allow-Origin', '*');
      newResponse.headers.set('Access-Control-Allow-Methods', 'GET, HEAD, OPTIONS');
      newResponse.headers.set('Access-Control-Allow-Headers', '*');
      
      return newResponse;
    } catch (error) {
      return new Response('Proxy error: ' + error.message, { 
        status: 500,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }
  }
}
