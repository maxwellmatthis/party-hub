// Get the stored answers and party ID from localStorage
const storedData = localStorage.getItem('public_party_answers');
if (!storedData) {
    window.location.href = '/';
}

const { partyId, answers } = JSON.parse(storedData);

document.getElementById('registration-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const salutation = document.getElementById('salutation').value.trim();
    const first = document.getElementById('first').value.trim();
    const last = document.getElementById('last').value.trim();
    const email = document.getElementById('email').value.trim();
    
    if (!first) {
        showError('First name is required');
        return;
    }
    
    const createBtn = document.getElementById('create-btn');
    createBtn.disabled = true;
    createBtn.textContent = 'Creating...';
    
    try {
        // Step 1: Create the guest
        const response = await fetch(`/guest/public_guest/${partyId}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                salutation,
                first,
                last,
                email
            })
        });
        
        const result = await response.json();
        
        if (!response.ok || !result.invitation_id) {
            showError(result.error || 'Failed to create guest');
            createBtn.disabled = false;
            createBtn.textContent = 'Create';
            return;
        }
        
        // Step 2: Save the answers using the regular save_answers endpoint
        const saveResponse = await fetch(`/invitation/${result.invitation_id}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ answers })
        });
        
        // Always clear stored data and redirect to invitation, even if save fails
        // (e.g., due to frozen party or deadline passed - user can still see their invitation)
        localStorage.removeItem('public_party_answers');
        window.location.href = `/${result.invitation_id}`;
        
    } catch (error) {
        console.error('Error creating guest:', error);
        showError('An error occurred. Please try again.');
        createBtn.disabled = false;
        createBtn.textContent = 'Create';
    }
});

function showError(message) {
    const errorDiv = document.getElementById('error-message');
    errorDiv.textContent = message;
    errorDiv.style.display = 'block';
}
