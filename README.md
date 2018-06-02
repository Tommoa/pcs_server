# pcs_server - A server for running programming competitions

This is the web-facing server side of PCS.

We currently force the use of HTTPS to clients, but an option to turn it off (or use both) will arrive in the coming weeks.

Actual interaction implementation has not begun yet, but the building blocks are there as we can currently host multiple clients and judges simultaneously.

Expected roadmap:
 - User handling backend
   - Most likely going to be managed by `diesel` and a SQL database (maybe PostGres?)
 - Fleshed out interactions with the judge
   - Currently connect and hang
 - Improved UI sent to users
   - Most likely going to be using `yew` in order to do this
