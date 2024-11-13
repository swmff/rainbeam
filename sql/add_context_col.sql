ALTER TABLE "xquestions" ADD COLUMN "context" TEXT DEFAULT '{}';
ALTER TABLE "xprofiles" ADD COLUMN "token_context" TEXT DEFAULT '[]';
ALTER TABLE "xcomments" ADD COLUMN "context" TEXT DEFAULT '{}';
