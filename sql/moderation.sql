INSERT INTO "xgroups" VALUES ('Moderators', '1', '["Helper"]');
INSERT INTO "xgroups" VALUES ('Admins', '2', '["Helper","Manager"]');
UPDATE "xprofiles" SET "group" = '1' WHERE "username" = 'USERNAME_HERE';
