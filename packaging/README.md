# Deployment

The package installs a static site at:

```text
/usr/share/webapps/wedding-std
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
   sudo /usr/share/doc/wedding-std/generate-self-signed-cert.sh
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

## Replace `wedding-website`

The old deployment includes the RSVP API and database. Remove its package
manually before the first `wedding-std` deploy:

```sh
sudo systemctl disable --now wedding-website-api.service
sudo tar -C / -czf /root/wedding-website-backup.tar.gz \
    var/lib/wedding-website \
    etc/wedding-website \
    etc/nginx/conf.d/wedding-website.conf
sudo pacman -R wedding-website
```

Do not use `pacman -Rns`; nginx remains required. The RSVP database remains in
`/var/lib/wedding-website`, but the backup protects it before the transition.

Then run `./deploy.sh` from the build workstation. If pacman saved the modified
old nginx file as `wedding-website.conf.pacsave`, the new package copies its TLS
certificate paths into `wedding-website-tls.conf`. Other old routing is not
carried forward.
