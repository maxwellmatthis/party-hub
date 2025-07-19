-- Add sample answers for Bob's invitation (choice types test)
UPDATE invitations SET invitation_block_answers = '{"2":[true,false,true,true],"3":2,"4":[false,true,false,true],"5":0}' WHERE id = '5d2032a1-1812-4b05-a57d-7ecdab03617a';

-- Add sample answers for Charlie's invitation (choice types test)
UPDATE invitations SET invitation_block_answers = '{"2":[false,true,true,false],"3":1,"4":[true,false,true,false],"5":2}' WHERE id = '3a3cbd95-e3a5-41fc-a648-3ff3a0b9627e';
