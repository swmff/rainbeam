#!/usr/bin/env bash
redis-cli KEYS "rbeam.app.reactions_count:*" | xargs redis-cli DEL
