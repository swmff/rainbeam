-- the minimum value for helpers is:
-- (1 << 2) | (1 << 3) | (1 << 4) | (1 << 14) | (1 << 15) | (1 << 25)
INSERT INTO
    "xgroups"
VALUES
    ('Helpers', '1', 33603612);

-- the minimum value for managers is:
-- (1 << 2) | (1 << 3) | (1 << 4) | (1 << 10) | (1 << 11) | (1 << 13) | (1 << 14) | (1 << 15) | (1 << 23) | (1 << 25)
INSERT INTO
    "xgroups"
VALUES
    ('Managers', '2', 42003484);

-- admins are granted administrator permissions:
-- (1 << 0) | (1 << 1)
INSERT INTO
    "xgroups"
VALUES
    ('Admins', '3', 3);

-- change profile group number:
UPDATE "xprofiles"
SET
    "group" = '1'
WHERE
    "username" = 'USERNAME_HERE';
