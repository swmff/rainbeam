ALTER TABLE "xprofiles"
DROP COLUMN "labels";

ALTER TABLE "xprofiles"
ADD COLUMN "labels" TEXT DEFAULT '[]';
