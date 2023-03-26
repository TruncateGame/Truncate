# Truncate

To build and serve a web client:

```bash
./.backstage/build-web-client.sh && python3 -m http.server -d web_client/ # or any webserver
```

Then load `localhost:8000?server=<server_addr>`
