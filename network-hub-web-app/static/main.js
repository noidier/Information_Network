document.addEventListener('DOMContentLoaded', function() {
    // Navigation
    const navLinks = document.querySelectorAll('.sidebar a');
    const sections = document.querySelectorAll('.content-section');
    
    navLinks.forEach(link => {
        link.addEventListener('click', function(e) {
            e.preventDefault();
            
            // Remove active class from all links and sections
            navLinks.forEach(link => link.classList.remove('active'));
            sections.forEach(section => section.classList.remove('active'));
            
            // Add active class to clicked link
            this.classList.add('active');
            
            // Show the corresponding section
            const sectionId = this.getAttribute('data-section');
            document.getElementById(sectionId).classList.add('active');
        });
    });
    
    // Set the first link as active by default
    navLinks[0].classList.add('active');
    
    // Hub Stats
    fetchHubStats();
    
    // Routes
    fetchRoutes();
    document.getElementById('route-form').addEventListener('submit', addRoute);
    
    // APIs
    fetchApis();
    document.getElementById('api-form').addEventListener('submit', registerApi);
    
    // API Requests
    document.getElementById('request-form').addEventListener('submit', sendApiRequest);
});

// Hub Stats
async function fetchHubStats() {
    try {
        const response = await fetch('/api/hub/stats');
        const stats = await response.json();
        
        document.getElementById('hub-scope').textContent = stats.scope;
        document.getElementById('api-count').textContent = stats.api_count;
        document.getElementById('interceptor-count').textContent = stats.interceptor_count;
    } catch (error) {
        console.error('Error fetching hub stats:', error);
    }
}

// Routes
async function fetchRoutes() {
    try {
        const response = await fetch('/api/routes');
        const routes = await response.json();
        
        const tableBody = document.querySelector('#routes-table tbody');
        tableBody.innerHTML = '';
        
        if (routes.length === 0) {
            const row = document.createElement('tr');
            row.innerHTML = '<td colspan="3" class="text-center">No routes configured</td>';
            tableBody.appendChild(row);
            return;
        }
        
        routes.forEach(route => {
            const row = document.createElement('tr');
            row.innerHTML = `
                <td>${route.path}</td>
                <td>${route.target}</td>
                <td>
                    <button class="action-btn delete" data-path="${route.path}">Delete</button>
                </td>
            `;
            tableBody.appendChild(row);
            
            // Add delete event listener
            row.querySelector('.delete').addEventListener('click', function() {
                deleteRoute(this.getAttribute('data-path'));
            });
        });
    } catch (error) {
        console.error('Error fetching routes:', error);
    }
}

async function addRoute(e) {
    e.preventDefault();
    
    const formData = new FormData(e.target);
    const route = {
        path: formData.get('path'),
        target: formData.get('target')
    };
    
    try {
        const response = await fetch('/api/routes', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(route)
        });
        
        if (response.ok) {
            e.target.reset();
            fetchRoutes();
        } else {
            console.error('Error adding route:', await response.text());
        }
    } catch (error) {
        console.error('Error adding route:', error);
    }
}

async function deleteRoute(path) {
    try {
        const response = await fetch(`/api/routes/${encodeURIComponent(path)}`, {
            method: 'DELETE'
        });
        
        if (response.ok) {
            fetchRoutes();
        } else {
            console.error('Error deleting route:', await response.text());
        }
    } catch (error) {
        console.error('Error deleting route:', error);
    }
}

// APIs
async function fetchApis() {
    try {
        const response = await fetch('/api/apis');
        const apis = await response.json();
        
        const tableBody = document.querySelector('#apis-table tbody');
        tableBody.innerHTML = '';
        
        if (apis.length === 0) {
            const row = document.createElement('tr');
            row.innerHTML = '<td colspan="2" class="text-center">No APIs registered</td>';
            tableBody.appendChild(row);
            return;
        }
        
        apis.forEach(api => {
            const row = document.createElement('tr');
            row.innerHTML = `
                <td>${api.path}</td>
                <td>
                    <button class="action-btn" data-path="${api.path}">Test</button>
                    <button class="action-btn delete" data-path="${api.path}">Delete</button>
                </td>
            `;
            tableBody.appendChild(row);
            
            // Add event listeners
            row.querySelector('.action-btn:not(.delete)').addEventListener('click', function() {
                const path = this.getAttribute('data-path');
                document.getElementById('request-path').value = path;
                // Switch to requests tab
                document.querySelector('[data-section="requests"]').click();
            });
            
            row.querySelector('.delete').addEventListener('click', function() {
                deleteApi(this.getAttribute('data-path'));
            });
        });
    } catch (error) {
        console.error('Error fetching APIs:', error);
    }
}

async function registerApi(e) {
    e.preventDefault();
    
    const formData = new FormData(e.target);
    const api = {
        path: formData.get('path'),
        response_data: formData.get('response_data')
    };
    
    try {
        const response = await fetch('/api/apis', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(api)
        });
        
        if (response.ok) {
            e.target.reset();
            fetchApis();
        } else {
            console.error('Error registering API:', await response.text());
        }
    } catch (error) {
        console.error('Error registering API:', error);
    }
}

async function deleteApi(path) {
    try {
        const response = await fetch(`/api/apis/${encodeURIComponent(path)}`, {
            method: 'DELETE'
        });
        
        if (response.ok) {
            fetchApis();
        } else {
            console.error('Error deleting API:', await response.text());
        }
    } catch (error) {
        console.error('Error deleting API:', error);
    }
}

// API Requests
async function sendApiRequest(e) {
    e.preventDefault();
    
    const formData = new FormData(e.target);
    const request = {
        path: formData.get('path'),
        data: formData.get('data') || ""
    };
    
    try {
        const response = await fetch('/api/request', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(request)
        });
        
        const result = await response.json();
        document.getElementById('response-output').textContent = 
            JSON.stringify(result, null, 2);
    } catch (error) {
        console.error('Error sending API request:', error);
        document.getElementById('response-output').textContent = 
            `Error: ${error.message}`;
    }
}