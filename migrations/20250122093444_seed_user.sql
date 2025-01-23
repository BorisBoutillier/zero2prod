-- Add migration script here
INSERT INTO users ( user_id,username, password_hash)
VALUES(
    '04bf7a74-eaf7-4e5c-afcb-3bc9073cac82',
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$Qf1zrP75j+Q1uZXwPPGAJQ$vOGa2/gDHegr8SOd+Bl2KiCKr4Z4bdp7m6fWXOd/D1s'
);
