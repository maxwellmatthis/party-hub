-- Add sample answers for Bob's invitation (lorem ipsum party)
UPDATE invitations SET invitation_block_answers = '{"5":[true,false,true,true,false,true],"7":4,"10":[false,true,false,false,false,true],"11":"Pizza, sushi, and vegetarian options please!"}' WHERE id = '3b20e6b4-4c0a-466d-b8b3-1ab103e0fe2e';

-- Add sample answers for Charlie's invitation (lorem ipsum party)
UPDATE invitations SET invitation_block_answers = '{"5":[false,true,false,true,true,false],"7":2,"10":[true,false,true,false,false,false],"11":"Something light and healthy would be great"}' WHERE id = '7645519a-e816-4156-858e-a55908d475e7';
