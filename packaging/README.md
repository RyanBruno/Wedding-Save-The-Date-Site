# Deployment

The package installs a static site at:

```text
/usr/share/webapps/wedding-website
```

It also installs nginx configuration at:

```text
/etc/nginx/conf.d/wedding-website.conf
/etc/nginx/wedding-website.headers.conf
```

No API, service, database, or runtime application is required.

## First server setup

1. Install nginx and the package.
2. Add `include conf.d/*.conf;` inside nginx's `http` block.
3. Disable nginx's default server block.
4. Generate a temporary certificate:

   ```sh
   sudo /usr/share/doc/wedding-website/generate-self-signed-cert.sh
   ```

5. Replace the certificate paths in the site configuration when a real
   certificate is available.
6. Validate and start nginx:

   ```sh
   sudo nginx -t
   sudo systemctl enable --now nginx.service
   ```

## Update on the server

```sh
./scripts/update-server.sh
```

This pulls `main`, builds the package, installs it, and reloads nginx.
