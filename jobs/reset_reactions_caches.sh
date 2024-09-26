#!/usr/bin/env bash
redis-cli KEYS "xsulib.sparkler.reactions_count:*" | xargs redis-cli DEL
