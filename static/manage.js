// DOM elements
const parties = document.querySelector("div#parties");
const main = document.querySelector("main");
const addPartyBtn = document.getElementById('add-party-btn');
const toastContainer = document.getElementById('toast-container');
const guests = document.querySelector("div#guests");
const addNewGuestBtn = document.getElementById('add-new-guest-btn');

// Toast notification utility
function showToast(message, type = 'info', duration = 4000) {
    const toastElement = templateToast.content.cloneNode(true);
    const toast = toastElement.querySelector('.toast');
    const messageSpan = toastElement.querySelector('.toast-message');
    const closeButton = toastElement.querySelector('.toast-close');

    messageSpan.textContent = message;
    toast.classList.add(type);
    toastContainer.appendChild(toastElement);
    setTimeout(() => toast.classList.add('show'), 100);

    const hideToast = () => {
        toast.classList.remove('show');
        setTimeout(() => {
            if (toast.parentNode) toast.parentNode.removeChild(toast);
        }, 300);
    };

    closeButton.addEventListener('click', hideToast);
    setTimeout(hideToast, duration);
}

// Templates
const templateError = document.querySelector("template#error");
const templatePartyLi = document.querySelector("template#party-li");
const templateEditParty = document.querySelector("template#edit-party");
const templateGuest = document.querySelector("template#guest");
const templateInvitationBlock = document.querySelector("template#invitation-block");
const templateModal = document.querySelector("template#add-guest-modal");
const templateGuestItem = document.querySelector("template#modal-guest-item");
const templateToast = document.querySelector("template#toast");
const templateEmptyState = document.querySelector("template#empty-state");
const templateGuestLi = document.querySelector("template#guest-li");
const templateEditGuest = document.querySelector("template#edit-guest");

// Global variables for modal and guest data
let currentPartyId = null;
let allGuests = [];
let currentPartyGuests = [];

let blockOrder = [];

function initializeBlockEditor(container, blocks) {
    container.innerHTML = '';
    blockOrder = [];

    addInsertionPoint(container, 0);

    blocks.forEach((blockData) => {
        // Use existing block ID if available, otherwise generate a new one
        const blockId = blockData.id || generateBlockId();
        blockOrder.push(blockId);

        const blockElement = createInvitationBlock(blockId, blockData);
        container.appendChild(blockElement);

        addInsertionPoint(container, blockOrder.length);
    });

    initializeDragAndDrop(container);
}

function createInvitationBlock(blockId, blockData = {}) {
    const blockElement = templateInvitationBlock.content.cloneNode(true);
    const blockDiv = blockElement.querySelector('.invitation-block');

    blockDiv.setAttribute('data-block-id', blockId);
    blockDiv.draggable = true;

    const typeSelect = blockElement.querySelector('.block-type-select');
    const contentTextarea = blockElement.querySelector('textarea');
    const optionsInput = blockElement.querySelector('input[type="text"]');
    const visibilitySelect = blockElement.querySelector('select[name="visibility"]');

    const template = blockData.template || 'p';
    typeSelect.value = template;

    // Handle content based on block type
    if (['single_choice', 'multiple_choice', 'text_input', 'number_input'].includes(template)) {
        // For question types, parse JSON content
        try {
            const questionData = JSON.parse(blockData.content || '{}');
            contentTextarea.value = questionData.label || '';
            if (questionData.options && Array.isArray(questionData.options)) {
                optionsInput.value = questionData.options.join(', ');
            }
            visibilitySelect.value = questionData.public ? 'public' : 'private';
        } catch (e) {
            // Fallback for old format or invalid JSON
            contentTextarea.value = blockData.content || '';
            if (blockData.options) optionsInput.value = blockData.options;
            if (blockData.visibility) visibilitySelect.value = blockData.visibility;
        }
    } else {
        // For non-question blocks, use content directly
        contentTextarea.value = blockData.content || '';
    }

    typeSelect.addEventListener('change', (e) => updateBlockVisibility(blockDiv, e.target.value));

    const deleteBtn = blockElement.querySelector('.block-delete');
    deleteBtn.addEventListener('click', () => deleteBlock(blockId));

    updateBlockVisibility(blockDiv, typeSelect.value);

    return blockElement;
}

function updateBlockVisibility(blockDiv, blockType) {
    const optionsInput = blockDiv.querySelector('input[type="text"]');
    const visibilitySelect = blockDiv.querySelector('select[name="visibility"]');

    const isChoiceQuestion = ['single_choice', 'multiple_choice'].includes(blockType);
    optionsInput.style.display = isChoiceQuestion ? 'block' : 'none';

    const isQuestion = ['single_choice', 'multiple_choice', 'text_input', 'number_input'].includes(blockType);
    visibilitySelect.style.display = isQuestion ? 'block' : 'none';
}

function addInsertionPoint(container, position) {
    const insertionElement = document.querySelector('template#block-insertion-point').content.cloneNode(true);
    const insertionDiv = insertionElement.querySelector('.block-insertion-point');
    const addButton = insertionElement.querySelector('.add-block-here');

    addButton.addEventListener('click', () => {
        insertBlockAtPosition(position);
    });

    container.appendChild(insertionElement);
}

function addInvitationBlock() {
    insertBlockAtPosition(blockOrder.length);
}

function insertBlockAtPosition(position) {
    const blockId = generateBlockId();
    blockOrder.splice(position, 0, blockId);

    const container = document.querySelector('div#invitation-blocks');
    rebuildBlockEditor(container);

    setTimeout(() => {
        const newBlock = container.querySelector(`[data-block-id="${blockId}"]`);
        if (newBlock) {
            const textarea = newBlock.querySelector('textarea');
            textarea?.focus();
        }
    }, 100);
}

function deleteBlock(blockId) {
    if (confirm('Are you sure you want to delete this block?')) {
        const index = blockOrder.indexOf(blockId);
        if (index > -1) {
            blockOrder.splice(index, 1);
        }

        const container = document.querySelector('div#invitation-blocks');
        rebuildBlockEditor(container);

        showToast('Block deleted', 'success');
    }
}

function rebuildBlockEditor(container) {
    const existingBlocks = {};

    container.querySelectorAll('.invitation-block').forEach(block => {
        const blockId = block.getAttribute('data-block-id');
        if (blockId) {
            existingBlocks[blockId] = getBlockData(block);
        }
    });

    container.innerHTML = '';
    addInsertionPoint(container, 0);

    blockOrder.forEach((blockId, index) => {
        const blockData = existingBlocks[blockId] || {};
        const blockElement = createInvitationBlock(blockId, blockData);
        container.appendChild(blockElement);
        addInsertionPoint(container, index + 1);
    });
}

function getBlockData(blockElement) {
    const typeSelect = blockElement.querySelector('.block-type-select');
    const contentTextarea = blockElement.querySelector('textarea');
    const optionsInput = blockElement.querySelector('input[type="text"]');
    const visibilitySelect = blockElement.querySelector('select[name="visibility"]');

    const template = typeSelect?.value || 'p';
    const rawContent = contentTextarea?.value || '';
    const rawOptions = optionsInput?.value || '';
    const visibility = visibilitySelect?.value || 'private';

    if (['single_choice', 'multiple_choice', 'text_input', 'number_input'].includes(template)) {
        const questionData = {
            label: rawContent,
            public: visibility === 'public'
        };

        if (['single_choice', 'multiple_choice'].includes(template) && rawOptions) {
            questionData.options = rawOptions.split(',').map(opt => opt.trim()).filter(opt => opt);
        }

        return {
            template: template,
            content: JSON.stringify(questionData)
        };
    } else {
        return {
            template: template,
            content: rawContent
        };
    }
}

function initializeDragAndDrop(container) {
    let draggedElement = null;
    let draggedBlockId = null;

    container.addEventListener('dragstart', (e) => {
        if (e.target.classList.contains('invitation-block')) {
            draggedElement = e.target;
            draggedBlockId = e.target.getAttribute('data-block-id');
            e.target.classList.add('dragging');
            e.dataTransfer.effectAllowed = 'move';
        }
    });

    container.addEventListener('dragend', (e) => {
        if (e.target.classList.contains('invitation-block')) {
            e.target.classList.remove('dragging');
            draggedElement = null;
            draggedBlockId = null;
        }
    });

    container.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'move';
    });

    container.addEventListener('drop', (e) => {
        e.preventDefault();

        if (!draggedBlockId) return;

        const dropTarget = e.target.closest('.invitation-block');
        if (dropTarget && dropTarget !== draggedElement) {
            const dropBlockId = dropTarget.getAttribute('data-block-id');
            reorderBlocks(draggedBlockId, dropBlockId);
        }
    });
}

function reorderBlocks(draggedBlockId, targetBlockId) {
    const draggedIndex = blockOrder.indexOf(draggedBlockId);
    const targetIndex = blockOrder.indexOf(targetBlockId);

    if (draggedIndex === -1 || targetIndex === -1) return;

    blockOrder.splice(draggedIndex, 1);

    const newTargetIndex = draggedIndex < targetIndex ? targetIndex : targetIndex + 1;
    blockOrder.splice(newTargetIndex, 0, draggedBlockId);

    const container = document.querySelector('div#invitation-blocks');
    rebuildBlockEditor(container);

    showToast('Block moved', 'success');
}

async function renderParty(partyId) {
    try {
        const response = await fetch(`/party/${partyId}`);
        if (!response.ok) throw new Error('Failed to load party');
        const partyDetails = await response.json();

        main.innerHTML = "";
        const p = templateEditParty.content.cloneNode(true);

        const nameInput = p.querySelector("input#party-name-input");
        nameInput.value = partyDetails.name;

        p.querySelector("#save-party-btn").addEventListener('click', () => saveParty(partyId));
        p.querySelector("#delete-party-btn").addEventListener('click', () => deleteParty(partyId));
        p.querySelector("#add-guest-btn").addEventListener('click', () => showAddGuestModal(partyId));
        p.querySelector("#add-block-btn").addEventListener('click', () => addInvitationBlock());

        const guestsContainer = p.querySelector("div#guests");
        partyDetails.guests.forEach(guest => {
            const guestElement = templateGuest.content.cloneNode(true);
            const guestDiv = guestElement.querySelector(".guest-item");

            guestElement.querySelector("span#guest-name").textContent = `${guest.first} ${guest.last}`.trim() || 'Unnamed Guest';

            const organizerButton = guestElement.querySelector("button#guest-organizer");
            const chevronUp = organizerButton.querySelector('#guest-promote');
            const chevronDown = organizerButton.querySelector('#guest-demote');

            chevronUp.style.display = guest.organizer ? 'none' : 'inline';
            chevronDown.style.display = guest.organizer ? 'inline' : 'none';

            guestElement.querySelector("button#guest-remove").addEventListener('click', async (e) => {
                e.preventDefault();
                e.stopPropagation();

                const response = await fetch(`/party/${partyId}/remove/${guest.id}`, {
                    method: 'DELETE',
                    credentials: 'same-origin'
                });

                if (response.ok) {
                    renderParty(partyId);
                } else {
                    console.error('Failed to remove guest:', await response.json());
                }
            });

            organizerButton.addEventListener('click', async (e) => {
                e.preventDefault();
                e.stopPropagation();

                const action = guest.organizer ? 'demote' : 'promote';
                const response = await fetch(`/party/${partyId}/${action}/${guest.id}`, {
                    method: 'POST',
                    credentials: 'same-origin'
                });

                if (response.ok) {
                    renderParty(partyId);
                } else {
                    console.error(`Failed to ${action} guest:`, await response.json());
                }
            });

            guestElement.querySelector("button#guest-copy-invitation").addEventListener('click', async (e) => {
                e.preventDefault();
                e.stopPropagation();

                const invitationUrl = `${window.location.origin}/${guest.invitation_id}`;

                try {
                    await navigator.clipboard.writeText(invitationUrl);
                    showToast(`Invitation link copied to clipboard!`, 'success');
                } catch (err) {
                    console.error('Failed to copy to clipboard:', err);
                    const textArea = document.createElement('textarea');
                    textArea.value = invitationUrl;
                    document.body.appendChild(textArea);
                    textArea.select();
                    try {
                        document.execCommand('copy');
                        showToast('Invitation link copied to clipboard!', 'success');
                    } catch (fallbackErr) {
                        console.error('Fallback copy failed:', fallbackErr);
                        showToast('Failed to copy invitation link', 'error');
                    }
                    document.body.removeChild(textArea);
                }
            });

            guestsContainer.appendChild(guestElement);
        });

        const invitationBlocksContainer = p.querySelector("div#invitation-blocks");
        const invitationBlocks = Array.isArray(partyDetails.invitation_blocks)
            ? partyDetails.invitation_blocks
            : [];

        initializeBlockEditor(invitationBlocksContainer, invitationBlocks);

        main.appendChild(p);
    } catch (error) {
        console.error('Error rendering party:', error);
        main.innerHTML = "";
        main.appendChild(templateError.content.cloneNode(true));
    }
}

async function renderParties() {
    try {
        const response = await fetch('/party');
        if (!response.ok) throw new Error('Failed to load parties');
        const myParties = await response.json();

        parties.innerHTML = "";

        myParties.forEach(party => {
            const pl = templatePartyLi.content.cloneNode(true);
            const nameBtn = pl.querySelector("button#party-name");
            nameBtn.textContent = party.name;
            nameBtn.addEventListener("click", () => renderParty(party.id));
            parties.appendChild(pl);
        });
    } catch (error) {
        console.error('Error loading parties:', error);
        parties.innerHTML = "";
        parties.appendChild(templateError.content.cloneNode(true));
    }
}

function setupCollapsibleSections() {
    const partiesSection = document.querySelector('aside section:first-child');
    const guestsSection = document.querySelector('aside section:last-child');

    const partiesCollapseBtn = document.getElementById('parties-collapse');
    const partiesExpandBtn = document.getElementById('parties-expand');
    const guestsCollapseBtn = document.getElementById('guests-collapse');
    const guestsExpandBtn = document.getElementById('guests-expand');

    partiesCollapseBtn?.addEventListener('click', () => partiesSection.classList.add('collapsed'));
    partiesExpandBtn?.addEventListener('click', () => partiesSection.classList.remove('collapsed'));
    guestsCollapseBtn?.addEventListener('click', () => guestsSection.classList.add('collapsed'));
    guestsExpandBtn?.addEventListener('click', () => guestsSection.classList.remove('collapsed'));
}

async function showAddGuestModal(partyId) {
    currentPartyId = partyId;

    try {
        const response = await fetch('/guest');
        if (!response.ok) throw new Error('Failed to fetch guests');
        allGuests = await response.json();

        const partyResponse = await fetch(`/party/${partyId}`);
        if (!partyResponse.ok) throw new Error('Failed to fetch party details');
        const partyDetails = await partyResponse.json();
        currentPartyGuests = partyDetails.guests.map(guest => guest.id);

        const modal = createAddGuestModal();
        document.body.appendChild(modal);

        populateGuestList();
        setupGuestSearch();

    } catch (error) {
        console.error('Error showing add guest modal:', error);
        showToast('Error loading guests. Please try again.', 'error');
    }
}

function createAddGuestModal() {
    const templateModal = document.querySelector("template#add-guest-modal");
    const modal = templateModal.content.cloneNode(true);
    const modalOverlay = modal.querySelector(".modal-overlay");

    modalOverlay.classList.add('show');

    const closeBtn = modal.querySelector(".modal-close");
    const cancelBtn = modal.querySelector(".btn-cancel");

    closeBtn.addEventListener('click', closeAddGuestModal);
    cancelBtn.addEventListener('click', closeAddGuestModal);

    modalOverlay.addEventListener('click', (e) => {
        if (e.target === modalOverlay) {
            closeAddGuestModal();
        }
    });

    return modal;
}

function closeAddGuestModal() {
    const modal = document.querySelector(".modal-overlay");
    if (modal) {
        modal.remove();
    }
    currentPartyId = null;
    allGuests = [];
    currentPartyGuests = [];
}

function populateGuestList(filteredGuests = null) {
    const guestList = document.querySelector("#modal-guest-list");
    if (!guestList) return;

    const guestsToShow = filteredGuests || allGuests;

    const availableGuests = guestsToShow.filter(guest => !currentPartyGuests.includes(guest.id));

    guestList.innerHTML = "";

    if (availableGuests.length === 0) {
        const emptyState = document.createElement('div');
        emptyState.className = 'guest-list-empty';
        emptyState.textContent = filteredGuests ? 'No guests found matching your search.' : 'No available guests to add.';
        guestList.appendChild(emptyState);
        return;
    }

    const templateGuestItem = document.querySelector("template#modal-guest-item");

    availableGuests.forEach(guest => {
        const guestItem = templateGuestItem.content.cloneNode(true);
        const guestDiv = guestItem.querySelector(".modal-guest-item");
        const guestName = guestItem.querySelector(".guest-name");
        const addBtn = guestItem.querySelector(".btn-add-guest");

        guestDiv.setAttribute('data-guest-id', guest.id);
        guestName.textContent = `${guest.first} ${guest.last}`.trim() || 'Unnamed Guest';

        addBtn.addEventListener('click', () => addGuestToParty(guest.id, `${guest.first} ${guest.last}`.trim()));

        guestList.appendChild(guestItem);
    });
}

function setupGuestSearch() {
    const searchInput = document.querySelector("#guest-search");
    if (!searchInput) return;

    searchInput.addEventListener('input', (e) => {
        const searchTerm = e.target.value.toLowerCase().trim();

        if (searchTerm === '') {
            populateGuestList();
        } else {
            const filteredGuests = allGuests.filter(guest => {
                const fullName = `${guest.first} ${guest.last}`.toLowerCase();
                return fullName.includes(searchTerm);
            });
            populateGuestList(filteredGuests);
        }
    });
}

async function addGuestToParty(guestId, guestName) {
    try {
        const response = await fetch(`/party/${currentPartyId}/add/${guestId}`, {
            method: 'POST',
            credentials: 'same-origin'
        });

        if (response.ok) {
            const partyId = currentPartyId;
            closeAddGuestModal();
            renderParty(partyId);
        } else {
            const error = await response.json();
            console.error('Failed to add guest:', error);
            showToast('Failed to add guest: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error adding guest:', error);
        showToast('Error adding guest. Please try again.', 'error');
    }
}

async function saveParty(partyId) {
    try {
        const nameInput = document.querySelector("input#party-name-input");
        if (!nameInput) {
            showToast('Error: Could not find party name input', 'error');
            return;
        }

        const partyName = nameInput.value.trim();
        if (!partyName) {
            showToast('Please enter a party name', 'warning');
            nameInput.focus();
            return;
        }

        const invitationBlocks = [];

        blockOrder.forEach(blockId => {
            const blockElement = document.querySelector(`[data-block-id="${blockId}"]`);
            if (blockElement) {
                const blockData = getBlockData(blockElement);
                blockData.id = blockId;
                invitationBlocks.push(blockData);
            }
        });

        const updateData = {
            name: partyName,
            invitation_blocks: JSON.stringify(invitationBlocks)
        };

        const response = await fetch(`/party/${partyId}/update`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            credentials: 'same-origin',
            body: JSON.stringify(updateData)
        });

        if (response.ok) {
            const _result = await response.json();
            showToast('Party saved successfully!', 'success');
            renderParties();
        } else {
            const error = await response.json();
            console.error('Failed to save party:', error);
            showToast('Failed to save party: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error saving party:', error);
        showToast('Error saving party. Please try again.', 'error');
    }
}

async function deleteParty(partyId) {
    const nameInput = document.querySelector("input#party-name-input");
    const partyName = nameInput ? nameInput.value : 'this party';
    const confirmed = confirm(`Are you sure you want to delete "${partyName}"? This action cannot be undone and will remove all guests and invitations.`);

    if (!confirmed) {
        return;
    }

    try {
        const response = await fetch(`/party/${partyId}/delete`, {
            method: 'DELETE',
            credentials: 'same-origin'
        });

        if (response.ok) {
            const result = await response.json();
            showToast('Party deleted successfully!', 'success');
            main.innerHTML = "";
            main.appendChild(templateEmptyState.content.cloneNode(true));
            renderParties();
        } else {
            const error = await response.json();
            console.error('Failed to delete party:', error);
            showToast('Failed to delete party: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error deleting party:', error);
        showToast('Error deleting party. Please try again.', 'error');
    }
}

async function createNewParty() {
    try {
        const response = await fetch('/party/new', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            credentials: 'same-origin'
        });

        if (response.ok) {
            const result = await response.json();
            renderParties();
            if (result.party_id) {
                renderParty(result.party_id);
            }
        } else {
            const error = await response.json();
            console.error('Failed to create party:', error);
            showToast('Failed to create party: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error creating party:', error);
        showToast('Error creating party. Please try again.', 'error');
    }
}

async function renderGuests() {
    try {
        const response = await fetch('/guest');
        if (!response.ok) throw new Error('Failed to load guests');
        const myGuests = await response.json();

        guests.innerHTML = "";

        myGuests.forEach(guest => {
            const gl = templateGuestLi.content.cloneNode(true);
            const nameBtn = gl.querySelector("button#guest-sidebar-name");
            // Display full name (first + last)
            const displayName = `${guest.first} ${guest.last}`.trim() || 'Unnamed Guest';
            nameBtn.textContent = displayName;
            nameBtn.addEventListener("click", () => renderGuest(guest.id));
            guests.appendChild(gl);
        });
    } catch (error) {
        console.error('Error loading guests:', error);
        guests.innerHTML = "";
        guests.appendChild(templateError.content.cloneNode(true));
    }
}

async function renderGuest(guestId) {
    try {
        const response = await fetch(`/guest/${guestId}`);
        if (!response.ok) throw new Error('Failed to load guest');
        const guestDetails = await response.json();

        main.innerHTML = "";
        const g = templateEditGuest.content.cloneNode(true);

        const salutationInput = g.querySelector("input#guest-edit-salutation");
        const firstInput = g.querySelector("input#guest-edit-first");
        const lastInput = g.querySelector("input#guest-edit-last");
        const emailInput = g.querySelector("input#guest-edit-email");
        const noteTextarea = g.querySelector("textarea#guest-edit-note");
        
        salutationInput.value = guestDetails.salutation || '';
        firstInput.value = guestDetails.first || '';
        lastInput.value = guestDetails.last || '';
        emailInput.value = guestDetails.email || '';
        noteTextarea.value = guestDetails.note || '';

        g.querySelector("#save-guest-btn").addEventListener('click', () => saveGuest(guestId));
        g.querySelector("#delete-guest-btn").addEventListener('click', () => deleteGuest(guestId));

        main.appendChild(g);
    } catch (error) {
        console.error('Error rendering guest:', error);
        main.innerHTML = "";
        main.appendChild(templateError.content.cloneNode(true));
    }
}

async function saveGuest(guestId) {
    try {
        const salutationInput = document.querySelector("input#guest-edit-salutation");
        const firstInput = document.querySelector("input#guest-edit-first");
        const lastInput = document.querySelector("input#guest-edit-last");
        const emailInput = document.querySelector("input#guest-edit-email");
        const noteTextarea = document.querySelector("textarea#guest-edit-note");
        
        if (!firstInput || !lastInput) {
            showToast('Error: Could not find guest input fields', 'error');
            return;
        }

        const firstName = firstInput.value.trim();
        const lastName = lastInput.value.trim();
        
        if (!firstName && !lastName) {
            showToast('Please enter at least a first or last name', 'warning');
            firstInput.focus();
            return;
        }

        const updateData = {
            salutation: salutationInput ? salutationInput.value.trim() : '',
            first: firstName,
            last: lastName,
            email: emailInput ? emailInput.value.trim() : '',
            note: noteTextarea ? noteTextarea.value.trim() : ''
        };

        const response = await fetch(`/guest/${guestId}/update`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            credentials: 'same-origin',
            body: JSON.stringify(updateData)
        });

        if (response.ok) {
            showToast('Guest saved successfully!', 'success');
            renderGuests();
        } else {
            const error = await response.json();
            console.error('Failed to save guest:', error);
            showToast('Failed to save guest: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error saving guest:', error);
        showToast('Error saving guest. Please try again.', 'error');
    }
}

async function deleteGuest(guestId) {
    const firstInput = document.querySelector("input#guest-edit-first");
    const lastInput = document.querySelector("input#guest-edit-last");
    const guestName = (firstInput && lastInput) 
        ? `${firstInput.value} ${lastInput.value}`.trim() || 'this guest'
        : 'this guest';
    const confirmed = confirm(`Are you sure you want to delete "${guestName}"? This action cannot be undone and will remove the guest from all parties.`);

    if (!confirmed) {
        return;
    }

    try {
        const response = await fetch(`/guest/${guestId}/delete`, {
            method: 'DELETE',
            credentials: 'same-origin'
        });

        if (response.ok) {
            showToast('Guest deleted successfully!', 'success');
            main.innerHTML = "";
            main.appendChild(templateEmptyState.content.cloneNode(true));
            renderGuests();
        } else {
            const error = await response.json();
            console.error('Failed to delete guest:', error);
            showToast('Failed to delete guest: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error deleting guest:', error);
        showToast('Error deleting guest. Please try again.', 'error');
    }
}

async function createNewGuest() {
    try {
        const response = await fetch('/guest/new', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            credentials: 'same-origin'
        });

        if (response.ok) {
            const result = await response.json();
            renderGuests();
            if (result.guest_id) {
                renderGuest(result.guest_id);
            }
        } else {
            const error = await response.json();
            console.error('Failed to create guest:', error);
            showToast('Failed to create guest: ' + (error.error || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Error creating guest:', error);
        showToast('Error creating guest. Please try again.', 'error');
    }
}

function generateBlockId() {
    // random UUID-like string without external libraries
    return 'block_' + 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
        const r = Math.random() * 16 | 0;
        const v = c === 'x' ? r : (r & 0x3 | 0x8);
        return v.toString(16);
    });
}

if (addPartyBtn) {
    addPartyBtn.addEventListener('click', createNewParty);
}

if (addNewGuestBtn) {
    addNewGuestBtn.addEventListener('click', createNewGuest);
}

// Initialize collapsible sections when the page loads
setupCollapsibleSections();

// Show empty state initially
main.appendChild(templateEmptyState.content.cloneNode(true));

renderParties();
renderGuests();

