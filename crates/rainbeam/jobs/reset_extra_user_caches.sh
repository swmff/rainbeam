#!/usr/bin/env bash
redis-cli KEYS "rbeam.app.friends_count:*" | xargs redis-cli DEL
redis-cli KEYS "rbeam.auth.followers_count:*" | xargs redis-cli DEL
redis-cli KEYS "rbeam.auth.following_count:*" | xargs redis-cli DEL
