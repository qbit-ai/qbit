# search
curl --request POST \
  --url https://api.tavily.com/search \
  --header 'Authorization: Bearer <token>' \
  --header 'Content-Type: application/json' \
  --data '
{
  "query": "who is Leo Messi?",
  "search_depth": "basic",
  "chunks_per_source": 3,
  "max_results": 1,
  "topic": "general",
  "time_range": null,
  "start_date": "2025-02-09",
  "end_date": "2025-12-29",
  "include_answer": false,
  "include_raw_content": false,
  "include_images": false,
  "include_image_descriptions": false,
  "include_favicon": false,
  "include_domains": [],
  "exclude_domains": [],
  "country": null,
  "auto_parameters": false,
  "include_usage": false
}
'

# extract
curl --request POST \
  --url https://api.tavily.com/extract \
  --header 'Authorization: Bearer <token>' \
  --header 'Content-Type: application/json' \
  --data '
{
  "urls": "https://en.wikipedia.org/wiki/Artificial_intelligence",
  "query": "<string>",
  "chunks_per_source": 3,
  "extract_depth": "basic",
  "include_images": false,
  "include_favicon": false,
  "format": "markdown",
  "timeout": "None",
  "include_usage": false
}
'

# crawl
curl --request POST \
  --url https://api.tavily.com/crawl \
  --header 'Authorization: Bearer <token>' \
  --header 'Content-Type: application/json' \
  --data '
{
  "url": "docs.tavily.com",
  "instructions": "Find all pages about the Python SDK",
  "chunks_per_source": 3,
  "max_depth": 1,
  "max_breadth": 20,
  "limit": 50,
  "select_paths": null,
  "select_domains": null,
  "exclude_paths": null,
  "exclude_domains": null,
  "allow_external": true,
  "include_images": false,
  "extract_depth": "basic",
  "format": "markdown",
  "include_favicon": false,
  "timeout": 150,
  "include_usage": false
}
'

# map
curl --request POST \
  --url https://api.tavily.com/map \
  --header 'Authorization: Bearer <token>' \
  --header 'Content-Type: application/json' \
  --data '
{
  "url": "docs.tavily.com",
  "instructions": "Find all pages about the Python SDK",
  "max_depth": 1,
  "max_breadth": 20,
  "limit": 50,
  "select_paths": null,
  "select_domains": null,
  "exclude_paths": null,
  "exclude_domains": null,
  "allow_external": true,
  "timeout": 150,
  "include_usage": false
}
'


