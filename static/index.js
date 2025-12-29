// Load and display invitations from localStorage
function loadInvitations() {
    const invitationsList = document.getElementById('invitations-list');
    const emptyState = document.getElementById('empty-state');
    
    // Get invitations from localStorage
    const invitations = JSON.parse(localStorage.getItem('party_hub_invitations') || '[]');
    
    if (invitations.length === 0) {
        emptyState.style.display = 'block';
        invitationsList.style.display = 'none';
    } else {
        emptyState.style.display = 'none';
        invitationsList.style.display = 'block';
        
        // Clear existing items
        invitationsList.innerHTML = '';
        
        // Sort by date (newest first)
        invitations.sort((a, b) => {
            const dateA = new Date(a.date || 0);
            const dateB = new Date(b.date || 0);
            return dateB - dateA;
        });
        
        // Render each invitation
        invitations.forEach((invitation, index) => {
            const item = document.createElement('div');
            item.className = 'invitation-item';
            
            const link = document.createElement('a');
            link.href = `/${invitation.invitationId}`;
            
            const name = document.createElement('div');
            name.className = 'invitation-name';
            name.textContent = invitation.name;
            
            const details = document.createElement('div');
            details.className = 'invitation-details';
            
            // Format date
            let formattedDate = 'No date';
            if (invitation.date) {
                const date = new Date(invitation.date);
                if (!isNaN(date.getTime())) {
                    formattedDate = date.toLocaleDateString(undefined, { 
                        year: 'numeric', 
                        month: '2-digit', 
                        day: '2-digit' 
                    });
                }
            }
            
            details.textContent = `${formattedDate} (${invitation.author} → ${invitation.invitee})`;
            
            link.appendChild(name);
            link.appendChild(details);
            item.appendChild(link);
            
            // Add delete button
            const deleteBtn = document.createElement('button');
            deleteBtn.className = 'delete-invitation-btn';
            deleteBtn.innerHTML = '<img src="/static/trash-2.svg" alt="Delete">';
            deleteBtn.title = 'Remove invitation';
            deleteBtn.addEventListener('click', (e) => {
                e.preventDefault();
                e.stopPropagation();
                deleteInvitation(invitation.invitationId);
            });
            item.appendChild(deleteBtn);
            
            invitationsList.appendChild(item);
        });
    }
}

function deleteInvitation(invitationId) {
    // Get current invitations
    const invitations = JSON.parse(localStorage.getItem('party_hub_invitations') || '[]');
    
    // Filter out the deleted invitation
    const updatedInvitations = invitations.filter(inv => inv.invitationId !== invitationId);
    
    // Save back to localStorage
    localStorage.setItem('party_hub_invitations', JSON.stringify(updatedInvitations));
    
    // Reload the list
    loadInvitations();
}

document.addEventListener('DOMContentLoaded', loadInvitations);
