# Deployment

The package installs a static site at:

```text
/usr/share/webapps/wedding-website
```

It also installs nginx configuration at:

```text
/etc/nginx/conf.d/wedding-website.conf
/etc/nginx/wedding-website.headers.conf
/etc/nginx/wedding-website-tls.conf
```

The package replaces the routing, cache, and security-header files on upgrade.
It preserves `wedding-website-tls.conf`, which contains machine-specific
certificate paths.

No API, service, database, or runtime application is required.

## First server setup

1. Install nginx and the package.
2. Add `include conf.d/*.conf;` inside nginx's `http` block.
3. Disable nginx's default server block.
4. Generate a temporary certificate:

   ```sh
   sudo /usr/share/doc/wedding-website/generate-self-signed-cert.sh
   ```

5. Replace both paths in `/etc/nginx/wedding-website-tls.conf` when a real
   certificate is available.
6. Validate and start nginx:

   ```sh
   sudo nginx -t
   sudo systemctl enable --now nginx.service
   ```

## Deploy from the build workstation

```sh
./build.sh
./deploy.sh
```

The server is too small for the release WASM build. `build.sh` creates the Arch
package in a local container. `deploy.sh` verifies, uploads, installs, and checks
that package through the public HTTPS URL.

Set another target with `--host`, and another public check URL with `--site`.
The equivalent environment variables are `WEDDING_DEPLOY_HOST` and
`WEDDING_SITE_URL`.
