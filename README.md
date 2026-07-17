# Quickbook Online API CLI tool

QBCLI is meant to access the [Quickbooks Online API](https://developer.intuit.com/app/developer/qbo/docs/learn/explore-the-quickbooks-online-api),
henceforth QBOAPI. This tool is useful for making queries and learning about the QBOAPI. You must
bring your own app credentials (client id and client secret) in order to use this tool. You can
find Intuit's docs for that [here](https://developer.intuit.com/app/developer/qbo/docs/get-started/start-developing-your-app).

This tool is very new. Contributions for missing functionality would be amazing.


# Setting up the tool.

1. cd to dev/localhost-keys
2. Run `./generate_keys.sh`
3. Make sure you have proper redirect_urls configureds for you app. Sandbox can use the default,
   which is `https://localhost:9999`. The key generation in the previous step also works for
   `https://0--1.nip.io:9999` and `https://127-0-0-1.nip.io:9999` if you want to try them in
   production.
4. Login to your QB account to get a token. This command will create a `default` profile for your
   token:
   ```sh
   qbcli auth login
   ```
5. Getting company info is a great way to make sure you can make a query.
   ```sh
   qbcli accounting company-info
   ```


# Using

The tools has profiles so that you can have mutliple tokens. Looks in the arguments for various subcommands for the `profile` argument.


# Contributing

Make and change, and send a PR. All PRs are assumed to be licensed identially to the main code base.

For bigger changes, please open an issue for discussion so that you aren't wasting your time. PRs
are fine for discussion as well. However, I would hate for you to wasted your time with a design
that will not be accepted.


# Privacy Policy

This tool (QBCLI) only uses locally supplied data (like defaults and application credentials) and
locally stored authentication bearer tokens. No data is either pulled from offsite or pushed to an
offsite location by QBCLI. You, as a user, are responsed to check this privacy policy for updates
each time you update QBCLI to see if there are any changes to this policy.

Here's a list of the data stored locally:
* Auth profiles - These profiles include auth tokens obtained by logging into the QBAPI via
  authenitcating and authorizing a user via the
  [QBOAPI authentication and authorization workflows](https://developer.intuit.com/app/developer/qbo/docs/develop/authentication-and-authorization).
  These profiles are stored in a keychain storage that uses the [SecretService](https://specifications.freedesktop.org/secret-service/)
  spec. I generally use an envinronment running Kwallet, which encrypts the stored data.
* Application defaults will be stored locally to facillitate usage of the API. For example, the
  default auth profile can be specified so that it need not be provided every time the tool is executed.
