/**
 * VOCAI: Vocab+AI — Frontend JavaScript
 * Handles flashcard interactions, SRS review, and AI generation
 */

// ============================================================
// Flashcard Management
// ============================================================

class FlashcardManager {
    constructor() {
        this.currentCard = null;
        this.isFlipped = false;
        this.reviewStartTime = null;
    }

    async loadFlashcard(cardId) {
        try {
            const response = await fetch(`/flashcards/${cardId}`);
            const contentType = response.headers.get('content-type');
            if (!contentType || !contentType.includes('application/json')) {
                throw new Error('Expected JSON response but got ' + (contentType || 'unknown'));
            }
            const card = await response.json();
            this.renderFlashcard(card);
            this.currentCard = card;
        } catch (error) {
            console.error('Error loading flashcard:', error);
            this.showError('Failed to load flashcard: ' + error.message);
        }
    }

    renderFlashcard(card) {
        const container = document.getElementById('flashcard-container');
        if (!container) return;

        container.innerHTML = `
            <div class="flashcard" id="flashcard" onclick="flashcardManager.flipCard()">
                <div class="flashcard__front" id="card-front">
                    <div class="flashcard__word">${card.word}</div>
                    ${card.phonetic ? `<div class="flashcard__phonetic">${card.phonetic}</div>` : ''}
                    ${card.part_of_speech ? `<span class="badge badge--info">${card.part_of_speech}</span>` : ''}
                    <p class="mt-md text-secondary">Click to reveal definition</p>
                </div>
                <div class="flashcard__back hidden" id="card-back">
                    ${card.image_url ? `<img src="${card.image_url}" alt="${card.word}" class="flashcard__image" />` : ''}
                    <div class="flashcard__definition">${card.definition}</div>
                    ${card.example_sentence ? `<div class="flashcard__example">"${card.example_sentence}"</div>` : ''}
                </div>
            </div>
        `;
    }

    flipCard() {
        const front = document.getElementById('card-front');
        const back = document.getElementById('card-back');
        
        if (!this.isFlipped) {
            front.classList.add('hidden');
            back.classList.remove('hidden');
            this.isFlipped = true;
            this.showReviewButtons();
        } else {
            front.classList.remove('hidden');
            back.classList.add('hidden');
            this.isFlipped = false;
            this.hideReviewButtons();
        }
    }

    showReviewButtons() {
        let buttons = document.getElementById('review-buttons');
        if (!buttons) {
            buttons = document.createElement('div');
            buttons.id = 'review-buttons';
            buttons.className = 'review-buttons mt-lg';
            buttons.innerHTML = `
                <button class="review-btn review-btn--again" onclick="flashcardManager.submitReview(0)">
                    Again<br><small>1m</small>
                </button>
                <button class="review-btn review-btn--hard" onclick="flashcardManager.submitReview(2)">
                    Hard<br><small>6m</small>
                </button>
                <button class="review-btn review-btn--good" onclick="flashcardManager.submitReview(4)">
                    Good<br><small>10m</small>
                </button>
                <button class="review-btn review-btn--easy" onclick="flashcardManager.submitReview(5)">
                    Easy<br><small>4d</small>
                </button>
            `;
            document.getElementById('flashcard-container').appendChild(buttons);
        }
        this.reviewStartTime = Date.now();
    }

    hideReviewButtons() {
        const buttons = document.getElementById('review-buttons');
        if (buttons) buttons.remove();
    }

    async submitReview(quality) {
        if (!this.currentCard) return;

        const responseTime = this.reviewStartTime ? Date.now() - this.reviewStartTime : null;

        try {
            const response = await fetch('/flashcards/review', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    flashcard_id: this.currentCard.id,
                    quality: quality,
                    response_time_ms: responseTime
                })
            });

            if (!response.ok) throw new Error('Failed to submit review');

            const result = await response.json();
            console.log('Review submitted:', result);
            
            // Show feedback
            this.showFeedback(quality);
            
            // Move to next card after delay
            setTimeout(() => {
                this.loadNextCard();
            }, 1500);

        } catch (error) {
            console.error('Error submitting review:', error);
            this.showError('Failed to submit review');
        }
    }

    showFeedback(quality) {
        const feedback = document.createElement('div');
        feedback.className = 'fade-in text-center mt-lg';
        
        let message, color;
        if (quality >= 4) {
            message = '✓ Excellent! Keep it up!';
            color = 'var(--success)';
        } else if (quality >= 3) {
            message = '✓ Good! Moving to next interval';
            color = 'var(--secondary)';
        } else if (quality >= 2) {
            message = '△ Okay, will review sooner';
            color = 'var(--warning)';
        } else {
            message = '✗ Will review again soon';
            color = 'var(--error)';
        }

        feedback.innerHTML = `<p style="color: ${color}; font-weight: 600;">${message}</p>`;
        
        const container = document.getElementById('flashcard-container');
        container.appendChild(feedback);

        setTimeout(() => feedback.remove(), 1500);
    }

    async loadNextCard() {
        try {
            const response = await fetch('/review/next');
            if (!response.ok) throw new Error('No more cards');
            
            const card = await response.json();
            this.isFlipped = false;
            this.loadFlashcard(card.id);
        } catch (error) {
            this.showCompleteMessage();
        }
    }

    showCompleteMessage() {
        const container = document.getElementById('flashcard-container');
        container.innerHTML = `
            <div class="text-center fade-in">
                <h2>🎉 Review Session Complete!</h2>
                <p class="mt-md">You've reviewed all due cards. Great job!</p>
                <a href="/dashboard" class="btn btn--primary mt-lg">Back to Dashboard</a>
            </div>
        `;
    }

    showError(message) {
        const container = document.getElementById('flashcard-container');
        if (container) {
            container.innerHTML = `
                <div class="text-center">
                    <p style="color: var(--error);">${message}</p>
                    <button class="btn btn--primary mt-md" onclick="location.reload()">Try Again</button>
                </div>
            `;
        }
    }
}

// ============================================================
// AI Flashcard Generation
// ============================================================

class AIGenerationManager {
    constructor() {
        this.isGenerating = false;
        this._cards = [];
    }

    async generateFlashcards(topic, count = 10, language = 'English', difficulty = 'intermediate') {
        if (this.isGenerating) return;

        this.isGenerating = true;
        this.showLoadingState();

        try {
            const formData = new URLSearchParams();
            formData.append('topic', topic);
            formData.append('count', count);
            formData.append('language', language);
            formData.append('difficulty', difficulty);

            const response = await fetch('/flashcards/generate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                body: formData.toString()
            });

            // Check content type before parsing
            const contentType = response.headers.get('content-type');
            if (!contentType || !contentType.includes('application/json')) {
                const text = await response.text();
                throw new Error(`Server returned ${response.status}: ${text.substring(0, 100)}`);
            }

            const result = await response.json();

            if (!result.success) {
                throw new Error(result.error || 'Generation failed');
            }

            this._cards = result.flashcards || [];
            this.renderGeneratedCards(this._cards);

        } catch (error) {
            console.error('Error generating flashcards:', error);
            this.showError('Failed to generate flashcards: ' + error.message);
        } finally {
            this.isGenerating = false;
        }
    }

    showLoadingState() {
        const container = document.getElementById('generated-cards-container');
        if (container) {
            container.innerHTML = `
                <div class="loading">
                    <div class="loading__spinner"></div>
                    <p>AI is generating your flashcards...</p>
                </div>
            `;
        }
    }

    renderGeneratedCards(cards) {
        const container = document.getElementById('generated-cards-container');
        if (!container) return;

        container.innerHTML = `
            <h3 class="mt-lg">Generated Flashcards</h3>
            <div class="grid grid--2col mt-md">
                ${cards.map((card, i) => `
                    <div class="card fade-in">
                        <h4 style="color: var(--primary-light);">${card.word}</h4>
                        ${card.phonetic ? `<p class="mono" style="font-size:.85rem">${card.phonetic}</p>` : ''}
                        ${card.part_of_speech ? `<span class="badge badge--info">${card.part_of_speech}</span>` : ''}
                        <p class="mt-sm">${card.definition}</p>
                        ${card.example_sentence ? `<p class="mt-sm" style="color: var(--secondary); font-style: italic; font-size:.9rem;">"${card.example_sentence}"</p>` : ''}
                        <button class="btn btn--secondary btn--sm mt-md" onclick="aiManager.saveCard(aiManager._cards[${i}])">
                            Save to Collection
                        </button>
                    </div>
                `).join('')}
            </div>
        `;
    }

    async saveCard(card) {
        try {
            const response = await fetch('/api/flashcards/save', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(card)
            });
            const result = await response.json();
            if (result.success) {
                alert(`"${card.word}" saved to your collection!`);
            } else {
                alert('Failed to save: ' + (result.error || 'Unknown error'));
            }
        } catch (error) {
            console.error('Error saving card:', error);
            alert('Failed to save card');
        }
    }

    showError(message) {
        const container = document.getElementById('generated-cards-container');
        if (container) {
            container.innerHTML = `<p style="color: var(--error);">${message}</p>`;
        }
    }
}

// ============================================================
// Vocabulary Island Management
// ============================================================

class IslandManager {
    async createIsland(name, topic, description) {
        try {
            const formData = new URLSearchParams();
            formData.append('name', name);
            formData.append('topic', topic);
            if (description) formData.append('description', description);

            const response = await fetch('/islands', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                },
                body: formData.toString()
            });

            if (!response.ok) throw new Error('Failed to create island');

            const result = await response.json();
            window.location.href = `/islands/${result.id}`;
            
        } catch (error) {
            console.error('Error creating island:', error);
            alert('Failed to create vocabulary island');
        }
    }
}

// ============================================================
// Initialize on DOM Ready
// ============================================================

const flashcardManager = new FlashcardManager();
const aiManager = new AIGenerationManager();
const islandManager = new IslandManager();

document.addEventListener('DOMContentLoaded', () => {
    // Auto-load first flashcard if on study page
    if (window.location.pathname.includes('/flashcards/')) {
        const cardId = window.location.pathname.split('/').pop();
        if (cardId && cardId !== 'flashcards') {
            flashcardManager.loadFlashcard(cardId);
        }
    }

    // Setup generation form
    const generateForm = document.getElementById('generate-form');
    if (generateForm) {
        generateForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const topic = document.getElementById('topic').value;
            const count = document.getElementById('count').value || 10;
            const language = document.getElementById('language').value || 'English';
            const difficulty = document.getElementById('difficulty').value || 'intermediate';

            await aiManager.generateFlashcards(topic, count, language, difficulty);
        });
    }

    // Setup island creation form
    const islandForm = document.getElementById('island-form');
    if (islandForm) {
        islandForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const name = document.getElementById('island-name').value;
            const topic = document.getElementById('island-topic').value;
            const description = document.getElementById('island-description').value;

            await islandManager.createIsland(name, topic, description);
        });
    }

    // Animate elements on scroll
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };

    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.classList.add('fade-in');
                observer.unobserve(entry.target);
            }
        });
    }, observerOptions);

    document.querySelectorAll('.card, .stats-card').forEach(el => {
        observer.observe(el);
    });
});
