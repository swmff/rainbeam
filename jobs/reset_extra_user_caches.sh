#!/usr/bin/env bash
redis-cli KEYS "xsulib.sparkler.friends_count:*" | xargs redis-cli DEL
redis-cli KEYS "xsulib.authman.followers_count:*" | xargs redis-cli DEL
redis-cli KEYS "xsulib.authman.following_count:*" | xargs redis-cli DEL
