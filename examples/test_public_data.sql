-- Add sample answers for Bob's invitation
UPDATE invitations SET invitation_block_answers = '{"2":[true,false,false],"3":"Pizza and burgers","4":"Looking forward to it","5":[true,false,false,false]}' WHERE id = '818205f4-fa59-4a8f-b55e-4f9e922fa593';

-- Add sample answers for Charlie's invitation  
UPDATE invitations SET invitation_block_answers = '{"2":[false,false,true],"3":"Sushi and salads","4":"Cannot wait","5":[false,true,false,false]}' WHERE id = '996688c4-096f-4561-8706-37ab4beee808';
