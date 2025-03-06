ALTER TABLE "xprofiles"
ADD COLUMN "question_count" TEXT DEFAULT '0';

ALTER TABLE "xprofiles"
ADD COLUMN "response_count" TEXT DEFAULT '0';

ALTER TABLE "xprofiles"
ADD COLUMN "notification_count" TEXT DEFAULT '0';

ALTER TABLE "xprofiles"
ADD COLUMN "inbox_count" TEXT DEFAULT '0';
