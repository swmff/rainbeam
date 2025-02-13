-- mysql, mariadb
ALTER TABLE "xgroups" MODIFY "permissions" INTEGER;

-- postgresql
ALTER TABLE "xgroups"
ALTER COLUMN "permissions" TYPE INTEGER;
