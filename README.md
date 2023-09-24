# oauth-proxy-rs-nginx

A minimal yet very fast and powerful implementation of [oauth-proxy](https://github.com/oauth2-proxy/oauth2-proxy) in rust+axum, configurable with nginx

Currently only github oauth is supported

## Installation

```sh
git clone https://github.com/CoolElectronics/oauth-proxy-rs-nginx
cd oauth-proxy-rs-nginx
cargo install --path .
```

For use with nginx, you will need to either own the enterprise edition or compile nginx with [nginx-jwt-module](https://github.com/max-lt/nginx-jwt-module)

Here is how to do that:

```sh
git clone https://github.com/max-lt/nginx-jwt-module
git clone https://github.com/nginx/nginx
cd nginx
./auto/configure --add-dynamic-module=../nginx-jwt-module
make
make install

# make sure /usr/local/nginx/ is in your PATH
```

## Usage

- You will need to generate a secure JWT secret key. `./keygen.sh` will do this for you.

To start the auth server, run `oauth-proxy-rs-nginx -k /path/to/oauth-proxy-rs-nginx/keys/secret.pem -p 3000 --host 0.0.0.0 --client-id your_github_oauth_client_id --client-secret your_github_oauth_client_secret --authorized-users authorized_user_ids --authorized-orgs authorized_org_ids --authorized-domain yourserver.com`

```
Usage: oauth-proxy-rs-nginx [options]

Options:
        --authorized-users
                        comma separated list of github user IDs (find uid at
                        https://api.github.com/users/your_username)
        --authorized-orgs
                        comma separated list of github organization IDs (find
                        uid at https://api.github.com/orgs/your_organization)
        --client-secret
                        oauth client secret
        --client-id     oauth client ID
    -k, --key           set path to JWT secret
    -p, --port 8080     port to bind to
        --host 0.0.0.0  host to bind to
    -h, --help          print this help menu
```

To start proxying requests, edit your nginx config to check against the auth-server. Here is an example:

```nginx
load_module /usr/local/nginx/modules/ngx_http_auth_jwt_module.so;
http {
    server {
        server_name auth.yourserver.com;

	    location / {
        	proxy_pass http://127.0.0.1:3000; # same port as you specified in the CLI
    	}
        listen 80 ssl;
    }

    server {
        server_name restricted_endpoint.yourserver.com;

        auth_jwt_key /path/to/oauth-proxy-rs-nginx/keys/secret.pem file;
        auth_jwt off;


        error_page 401 /oauth-proxy-rs-nginx-auth-failure;

	    location / {
            auth_jwt   $cookie_OAuth_Proxy_rs_token;
        	proxy_pass http://127.0.0.1:7999; # port that you want to proxy
    	}

        location = /oauth-proxy-rs-nginx-auth-failure {
            return 302 http://auth.yourserver.com/?callback=http://restricted_endpoint.yourserver.com/oauth-proxy-rs-nginx-set;
        }
        location = /oauth-proxy-rs-nginx-set {
            add_header Set-Cookie "OAuth_Proxy_rs_token=$arg_token;Path=/;Max-Age=86400";
            return 302 http://restricted_endpoint.yourserver.com;
        }
        listen 80 ssl;
    }
}
```
