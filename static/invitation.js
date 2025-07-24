// MVC Architecture for Party Invitation Form

// ===== MODEL =====
class InvitationModel {
    constructor() {
        this.answers = new Map();
        this.listeners = new Set();
        this.invitationData = null;
    }

    setAnswer(blockId, value) {
        this.answers.set(blockId, value);
        this.notifyListeners('answerChanged', { blockId, value });
    }

    getAnswer(blockId) {
        return this.answers.get(blockId);
    }

    getAllAnswers() {
        const result = {};
        for (const [blockId, answer] of this.answers) {
            result[blockId] = answer;
        }
        return result;
    }

    setInvitationData(data) {
        this.invitationData = data;
        this.notifyListeners('dataLoaded', data);
    }

    getInvitationData() {
        return this.invitationData;
    }

    getGuestName() {
        return this.invitationData ? this.invitationData.guest_name : '';
    }

    addListener(callback) {
        this.listeners.add(callback);
    }

    removeListener(callback) {
        this.listeners.delete(callback);
    }

    notifyListeners(event, data) {
        this.listeners.forEach(callback => callback(event, data));
    }
}

// ===== VIEW =====
class InvitationView {
    constructor() {
        this.invitation_section = document.querySelector("section#invitation");
        this.templates = {
            error: document.querySelector("template#error"),
            multiple_choice: document.querySelector("template#multiple-choice"),
            multiple_choice_item: document.querySelector("template#multiple-choice-item"),
            single_choice: document.querySelector("template#single-choice"),
            single_choice_item: document.querySelector("template#single-choice-item"),
            text_input: document.querySelector("template#text-input"),
            number_input: document.querySelector("template#number-input"),
            public_stats: document.querySelector("template#public-stats"),
            response_item: document.querySelector("template#response-item"),
            status_saving: document.querySelector("template#status-saving"),
            status_saved: document.querySelector("template#status-saved"),
            status_save: document.querySelector("template#status-save"),
            error_save_failed: document.querySelector("template#error-save-failed"),
            error_no_invitation_id: document.querySelector("template#error-no-invitation-id"),
            error_save_generic: document.querySelector("template#error-save-generic")
        };
    }

    // Personalize content by replacing {{name}} with guest name
    personalizeContent(content, guestName) {
        if (typeof content === 'string') {
            return content.replace(/\{\{name\}\}/g, guestName || '');
        }
        return content;
    }

    render(invitation_blocks, invitation_block_answers, other_guests_answers, guestName, isOrganizer, onInputChange) {
        if (invitation_blocks.length < 1) return;
        this.invitation_section.innerHTML = "";
        invitation_blocks.forEach((block, i) => {
            const blockId = block.id || i.toString(); // Use block ID if available, fallback to index
            const answer = invitation_block_answers.hasOwnProperty(blockId) ? invitation_block_answers[blockId] : null;
            const div = document.createElement("div");
            div.classList.add("block");
            div.appendChild(this.createBlock(block.template, block.content, answer, blockId, other_guests_answers, guestName, isOrganizer, onInputChange));
            this.invitation_section.appendChild(div);
        });
    }

    createBlock(template, content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange) {
        if (['h1', 'h2', 'h3', 'p', 'code'].includes(template)) {
            const el = document.createElement(template);
            el.textContent = this.personalizeContent(content, guestName);
            return el;
        } else {
            switch (template) {
                case 'multiple_choice':
                    return this.createMultipleChoice(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange);
                case 'single_choice':
                    return this.createSingleChoice(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange);
                case 'text_input':
                    return this.createTextInput(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange);
                case 'number_input':
                    return this.createNumberInput(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange);
                default:
                    return this.templates.error.content.cloneNode(true);
            }
        }
    }

    createMultipleChoice(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange) {
        const mc = this.templates.multiple_choice.content.cloneNode(true);
        let mcContent;
        try {
            mcContent = typeof content === 'string' ? JSON.parse(content) : content;
        } catch {
            mcContent = { label: content, options: ["Yes", "No"] };
        }

        mc.querySelector("label").textContent = this.personalizeContent(mcContent.label, guestName);
        const ul = mc.querySelector('ul');

        const currentAnswer = answer_data || [];
        const isPublic = mcContent.public === true;

        // Calculate stats if this block is public or user is organizer
        let optionCounts = [];
        let optionGuestNames = []; // Track which guests selected each option
        if ((isPublic || isOrganizer) && other_guests_answers) {
            optionCounts = new Array(mcContent.options.length).fill(0);
            optionGuestNames = new Array(mcContent.options.length).fill().map(() => []);

            // Include other guests' answers
            other_guests_answers.forEach(guestAnswers => {
                const blockAnswerData = guestAnswers[blockId];
                if (blockAnswerData) {
                    const blockAnswer = blockAnswerData.answer;
                    const guestName = blockAnswerData.guest_name;
                    
                    if (Array.isArray(blockAnswer)) {
                        blockAnswer.forEach((selected, optionIndex) => {
                            if (selected && optionIndex < optionCounts.length) {
                                optionCounts[optionIndex]++;
                                optionGuestNames[optionIndex].push(guestName);
                            }
                        });
                    }
                }
            });

            // Include current user's answer
            if (Array.isArray(currentAnswer)) {
                currentAnswer.forEach((selected, optionIndex) => {
                    if (selected && optionIndex < optionCounts.length) {
                        optionCounts[optionIndex]++;
                    }
                });
            }
        }

        mcContent.options.forEach((o, i) => {
            const mci = this.templates.multiple_choice_item.content.cloneNode(true);
            const li = mci.querySelector('li');
            const checkbox = li.querySelector('input');
            const span = li.querySelector('span');

            // Set option text with public stats and guest names if available
            if ((isPublic || isOrganizer) && optionCounts.length > 0) {
                const count = optionCounts[i] || 0;
                const names = optionGuestNames[i] || [];
                if (names.length > 0) {
                    span.textContent = `${o} (${count}) - ${names.join(', ')}`;
                } else {
                    span.textContent = `${o} (${count})`;
                }
            } else {
                span.textContent = o;
            }

            checkbox.checked = !!currentAnswer[i];

            const updateAnswer = () => {
                // Get all checkboxes for this multiple choice question
                const allCheckboxes = ul.querySelectorAll('input[type="checkbox"]');
                const newAnswers = Array.from(allCheckboxes).map(cb => cb.checked);
                onInputChange(blockId, newAnswers);

                // Update stats in real-time
                updateStatsDisplay();
            };

            const updateStatsDisplay = () => {
                if ((isPublic || isOrganizer) && optionCounts.length > 0) {
                    // Recalculate counts including current selections
                    const updatedCounts = new Array(mcContent.options.length).fill(0);
                    const updatedGuestNames = new Array(mcContent.options.length).fill().map(() => []);

                    // Include other guests' answers
                    other_guests_answers.forEach(guestAnswers => {
                        const blockAnswerData = guestAnswers[blockId];
                        if (blockAnswerData) {
                            const blockAnswer = blockAnswerData.answer;
                            const guestName = blockAnswerData.guest_name;
                            
                            if (Array.isArray(blockAnswer)) {
                                blockAnswer.forEach((selected, optionIndex) => {
                                    if (selected && optionIndex < updatedCounts.length) {
                                        updatedCounts[optionIndex]++;
                                        updatedGuestNames[optionIndex].push(guestName);
                                    }
                                });
                            }
                        }
                    });

                    // Include current user's selections
                    const currentSelections = Array.from(ul.querySelectorAll('input[type="checkbox"]')).map(cb => cb.checked);
                    currentSelections.forEach((selected, optionIndex) => {
                        if (selected && optionIndex < updatedCounts.length) {
                            updatedCounts[optionIndex]++;
                        }
                    });

                    // Update all span texts with new counts and guest names
                    mcContent.options.forEach((o, i) => {
                        const currentSpan = ul.querySelectorAll('span')[i];
                        if (currentSpan) {
                            const count = updatedCounts[i] || 0;
                            const names = updatedGuestNames[i] || [];
                            if (names.length > 0) {
                                currentSpan.textContent = `${o} (${count}) - ${names.join(', ')}`;
                            } else {
                                currentSpan.textContent = `${o} (${count})`;
                            }
                        }
                    });
                }
            };

            checkbox.addEventListener('change', updateAnswer);

            li.addEventListener('click', function (e) {
                if (e.target !== checkbox) {
                    checkbox.checked = !checkbox.checked;
                    updateAnswer();
                }
            });

            ul.appendChild(li);
        });
        return mc;
    }

    createSingleChoice(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange) {
        const sc = this.templates.single_choice.content.cloneNode(true);
        let scContent;
        try {
            scContent = typeof content === 'string' ? JSON.parse(content) : content;
        } catch {
            scContent = { label: content, options: ["Yes", "No"] };
        }

        sc.querySelector("label").textContent = this.personalizeContent(scContent.label, guestName);
        const ul = sc.querySelector('ul');

        const currentAnswer = answer_data !== undefined && answer_data !== null ? answer_data : -1; // Single choice uses index, -1 means no selection
        const isPublic = scContent.public === true;
        const radioGroupName = `single_choice_${blockId}`;

        // Calculate stats if this block is public or user is organizer
        let optionCounts = [];
        let optionGuestNames = []; // Track which guests selected each option
        if ((isPublic || isOrganizer) && other_guests_answers) {
            optionCounts = new Array(scContent.options.length).fill(0);
            optionGuestNames = new Array(scContent.options.length).fill().map(() => []);

            // Include other guests' answers
            other_guests_answers.forEach(guestAnswers => {
                const blockAnswerData = guestAnswers[blockId];
                if (blockAnswerData) {
                    const blockAnswer = blockAnswerData.answer;
                    const guestName = blockAnswerData.guest_name;
                    
                    if (typeof blockAnswer === 'number' && blockAnswer >= 0 && blockAnswer < optionCounts.length) {
                        optionCounts[blockAnswer]++;
                        optionGuestNames[blockAnswer].push(guestName);
                    }
                }
            });

            // Include current user's answer
            if (typeof currentAnswer === 'number' && currentAnswer >= 0 && currentAnswer < optionCounts.length) {
                optionCounts[currentAnswer]++;
            }
        }

        scContent.options.forEach((o, i) => {
            const sci = this.templates.single_choice_item.content.cloneNode(true);
            const li = sci.querySelector('li');
            const radio = li.querySelector('input');
            const span = li.querySelector('span');

            // Set unique name for radio group
            radio.name = radioGroupName;
            radio.value = i;

            // Set option text with public stats and guest names if available
            if ((isPublic || isOrganizer) && optionCounts.length > 0) {
                const count = optionCounts[i] || 0;
                const names = optionGuestNames[i] || [];
                if (names.length > 0) {
                    span.textContent = `${o} (${count}) - ${names.join(', ')}`;
                } else {
                    span.textContent = `${o} (${count})`;
                }
            } else {
                span.textContent = o;
            }

            radio.checked = currentAnswer === i;

            const updateAnswer = () => {
                const selectedRadio = ul.querySelector('input[type="radio"]:checked');
                const selectedIndex = selectedRadio ? parseInt(selectedRadio.value) : -1;
                onInputChange(blockId, selectedIndex);

                // Update stats in real-time
                updateStatsDisplay();
            };

            const updateStatsDisplay = () => {
                if ((isPublic || isOrganizer) && optionCounts.length > 0) {
                    // Recalculate counts including current selection
                    const updatedCounts = new Array(scContent.options.length).fill(0);
                    const updatedGuestNames = new Array(scContent.options.length).fill().map(() => []);

                    // Include other guests' answers
                    other_guests_answers.forEach(guestAnswers => {
                        const blockAnswerData = guestAnswers[blockId];
                        if (blockAnswerData) {
                            const blockAnswer = blockAnswerData.answer;
                            const guestName = blockAnswerData.guest_name;
                            
                            if (typeof blockAnswer === 'number' && blockAnswer >= 0 && blockAnswer < updatedCounts.length) {
                                updatedCounts[blockAnswer]++;
                                updatedGuestNames[blockAnswer].push(guestName);
                            }
                        }
                    });

                    // Include current user's selection
                    const selectedRadio = ul.querySelector('input[type="radio"]:checked');
                    if (selectedRadio) {
                        const selectedIndex = parseInt(selectedRadio.value);
                        if (selectedIndex >= 0 && selectedIndex < updatedCounts.length) {
                            updatedCounts[selectedIndex]++;
                        }
                    }

                    // Update all span texts with new counts and guest names
                    scContent.options.forEach((o, i) => {
                        const currentSpan = ul.querySelectorAll('span')[i];
                        if (currentSpan) {
                            const count = updatedCounts[i] || 0;
                            const names = updatedGuestNames[i] || [];
                            if (names.length > 0) {
                                currentSpan.textContent = `${o} (${count}) - ${names.join(', ')}`;
                            } else {
                                currentSpan.textContent = `${o} (${count})`;
                            }
                        }
                    });
                }
            };

            radio.addEventListener('change', updateAnswer);

            li.addEventListener('click', function (e) {
                if (e.target !== radio) {
                    radio.checked = true;
                    updateAnswer();
                }
            });

            ul.appendChild(li);
        });
        return sc;
    }

    createTextInput(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange) {
        const ti = this.templates.text_input.content.cloneNode(true);
        const textInput = ti.querySelector("input");

        let textContent;
        try {
            textContent = typeof content === 'string' ? JSON.parse(content) : content;
            // Handle case where content is an array (from YAML parsing)
            if (Array.isArray(textContent) && textContent.length > 0) {
                textContent = textContent[0];
            }
        } catch {
            textContent = { label: content };
        }

        // Handle both string and object content
        const label = typeof textContent === 'object' ? textContent.label : textContent;
        const isPublic = typeof textContent === 'object' && textContent.public === true;

        ti.querySelector("label").textContent = this.personalizeContent(label, guestName);
        textInput.value = answer_data || '';

        textInput.addEventListener('input', () => {
            onInputChange(blockId, textInput.value);
        });

        // Add public stats if enabled or user is organizer
        if ((isPublic || isOrganizer) && other_guests_answers) {
            const otherAnswers = [];
            other_guests_answers.forEach(guestAnswers => {
                const blockAnswerData = guestAnswers[blockId];
                if (blockAnswerData) {
                    const blockAnswer = blockAnswerData.answer;
                    const guestName = blockAnswerData.guest_name;
                    
                    if (blockAnswer && typeof blockAnswer === 'string' && blockAnswer.trim() !== '') {
                        otherAnswers.push({ answer: blockAnswer, guest_name: guestName });
                    }
                }
            });

            if (otherAnswers.length > 0) {
                const statsDiv = this.templates.public_stats.content.cloneNode(true);
                const responsesContainer = statsDiv.querySelector('.other-responses');

                otherAnswers.forEach(({ answer, guest_name }) => {
                    const responseItem = this.templates.response_item.content.cloneNode(true);
                    responseItem.querySelector('.response-item').textContent = `${answer} - ${guest_name}`;
                    responsesContainer.appendChild(responseItem);
                });

                ti.appendChild(statsDiv);
            }
        }

        return ti;
    }

    createNumberInput(content, answer_data, blockId, other_guests_answers, guestName, isOrganizer, onInputChange) {
        const ni = this.templates.number_input.content.cloneNode(true);
        const numberInput = ni.querySelector("input");

        let numberContent;
        try {
            numberContent = typeof content === 'string' ? JSON.parse(content) : content;
            // Handle case where content is an array (from YAML parsing)
            if (Array.isArray(numberContent) && numberContent.length > 0) {
                numberContent = numberContent[0];
            }
        } catch {
            numberContent = { label: content };
        }

        // Handle both string and object content
        const label = typeof numberContent === 'object' ? numberContent.label : numberContent;
        const isPublic = typeof numberContent === 'object' && numberContent.public === true;

        ni.querySelector("label").textContent = this.personalizeContent(label, guestName);
        numberInput.value = answer_data || '';

        numberInput.addEventListener('input', () => {
            onInputChange(blockId, numberInput.value);
        });

        // Add public stats if enabled or user is organizer
        if ((isPublic || isOrganizer) && other_guests_answers) {
            const otherAnswers = [];
            other_guests_answers.forEach(guestAnswers => {
                const blockAnswerData = guestAnswers[blockId];
                if (blockAnswerData) {
                    const blockAnswer = blockAnswerData.answer;
                    const guestName = blockAnswerData.guest_name;
                    
                    if (blockAnswer && typeof blockAnswer === 'string' && blockAnswer.trim() !== '') {
                        otherAnswers.push({ answer: blockAnswer, guest_name: guestName });
                    }
                }
            });

            if (otherAnswers.length > 0) {
                const statsDiv = this.templates.public_stats.content.cloneNode(true);
                const responsesContainer = statsDiv.querySelector('.other-responses');

                otherAnswers.forEach(({ answer, guest_name }) => {
                    const responseItem = this.templates.response_item.content.cloneNode(true);
                    responseItem.querySelector('.response-item').textContent = `${answer} - ${guest_name}`;
                    responsesContainer.appendChild(responseItem);
                });

                ni.appendChild(statsDiv);
            }
        }

        return ni;
    }

    showSaveStatus(status, message) {
        const saveButton = document.querySelector("#form-save");
        if (!saveButton) return;

        switch (status) {
            case 'saving':
                saveButton.disabled = true;
                saveButton.textContent = this.templates.status_saving.content.textContent;
                break;
            case 'success':
                saveButton.textContent = this.templates.status_saved.content.textContent;
                saveButton.style.background = "#28a745";
                setTimeout(() => {
                    saveButton.textContent = this.templates.status_save.content.textContent;
                    saveButton.style.background = "";
                    saveButton.disabled = false;
                }, 2000);
                break;
            case 'error':
                saveButton.textContent = this.templates.status_save.content.textContent;
                saveButton.disabled = false;
                alert(message || this.templates.error_save_failed.content.textContent);
                break;
        }
    }
}

// ===== CONTROLLER =====
class InvitationController {
    constructor() {
        this.model = new InvitationModel();
        this.view = new InvitationView();
        this.init();
    }

    init() {
        // Set up model listeners
        this.model.addListener((event, data) => {
            if (event === 'dataLoaded') {
                this.renderInvitation();
            }
        });

        // Set up save button
        this.setupSaveButton();

        // Load invitation data
        this.loadInvitationData();
    }

    async loadInvitationData() {
        try {
            const data = await this.getDetails();
            this.model.setInvitationData(data);

            // Initialize model with existing answers
            if (data.invitation_block_answers) {
                Object.entries(data.invitation_block_answers).forEach(([blockId, answer]) => {
                    this.model.setAnswer(blockId, answer);
                });
            }
        } catch (error) {
            console.error('Failed to load invitation data:', error);
        }
    }

    renderInvitation() {
        const data = this.model.getInvitationData();
        if (!data) return;

        this.view.render(
            data.invitation_blocks,
            data.invitation_block_answers || {},
            data.other_guests_answers || [],
            data.guest_name, // Pass guest name to view
            data.is_organizer || false, // Pass organizer status to view
            (blockId, value) => this.model.setAnswer(blockId, value)
        );
    }

    setupSaveButton() {
        const saveButton = document.querySelector("#form-save");
        if (saveButton) {
            saveButton.addEventListener('click', () => this.saveAnswers());
        }
    }

    async getDetails() {
        const path = window.location.pathname.split('/').filter(Boolean);
        const invitationId = path.length > 0 ? path[0] : null;
        if (invitationId) {
            try {
                const response = await fetch(`/details/${invitationId}`);
                if (!response.ok) throw new Error('Network response was not ok');
                return await response.json();
            } catch (_error) { }
        }
        return { invitation_blocks: [], invitation_block_answers: {}, other_guests_answers: [] };
    }

    async saveAnswers() {
        const path = window.location.pathname.split('/').filter(Boolean);
        const invitationId = path.length > 0 ? path[0] : null;

        if (!invitationId) {
            this.view.showSaveStatus('error', this.view.templates.error_no_invitation_id.content.textContent);
            return;
        }

        try {
            this.view.showSaveStatus('saving');

            const response = await fetch(`/details/${invitationId}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ answers: this.model.getAllAnswers() })
            });

            const result = await response.json();

            if (response.ok) {
                this.view.showSaveStatus('success');
            } else {
                throw new Error(result.error || 'Save failed');
            }
        } catch (error) {
            console.error('Error saving answers:', error);
            this.view.showSaveStatus('error', error.message);
        }
    }
}

// Global instances (for backward compatibility)
const model = new InvitationModel();
const userAnswers = model.answers; // Backward compatibility

// Initialize the application
document.addEventListener('DOMContentLoaded', () => {
    new InvitationController();
});
