# It's Not Safe, It's Not Fast--It's Simple

Frequently, I will develop web applications locally that utilize a frontend
framework like React.js or Yew to talk to a RESTful API stood up by Django or
hyper.rs.

In the absence of production reverse proxies, like Nginx, serving these files
in development is a huge pain. Servers like `see` or `miniserv` can't reverse
proxy, and also can't set custom headers to allow unsafe CORS policies.
Additionally, development servers like `trunk` don't easily allow one to serve
static files.

When this project is completed, a developer will be able to write a simple
YAML file to configure reverse proxies, like below:

```
proxy:
  "/api": "http://localhost:3000/api"
root: pkg
bind: "locahost:8080"
```

When a developer runs `dev-prox` in the same directory as this YAML file,
requests to `http://localhost:8080/api` will be proxied to
`http://localhost:3000/api`, and requests to `http://localhost:8080/` will
serve the contents of the files in `./pkg` (and subdirectories).
